use super::{datapath::Datapath, mem};
use crate::datapath::{CornflakesSegment, MetadataStatus};
use ahash::AHashMap;
use color_eyre::eyre::{bail, Result, WrapErr};
#[cfg(feature = "profiler")]
use demikernel;
use hashbrown::HashMap;
use std::collections::HashSet;
pub type MempoolID = u32;
pub fn align_to_pow2(x: usize) -> usize {
    #[cfg(feature = "profiler")]
    demikernel::timer!("align to pow2");
    if x & (x - 1) == 0 {
        return x + (x == 0) as usize;
    }
    let mut size = x;
    size -= 1;
    size |= size >> 1;
    size |= size >> 2;
    size |= size >> 4;
    size |= size >> 8;
    size |= size >> 16;
    size |= size >> 32;
    size += 1;
    size += (size == 0) as usize;
    size
}

pub fn align_up(x: usize, align_size: usize) -> usize {
    let divisor = x / align_size;
    if (divisor * align_size) < x {
        return (divisor + 1) * align_size;
    } else {
        assert!(divisor * align_size == x);
        return x;
    }
}

/// Trait that datapath's can implement to pr
pub trait DatapathMemoryPool {
    type DatapathImpl: Datapath;
    type RegistrationContext;

    /// Returns CornflakesSegment associated with particular page.
    fn get_segment_info(&self, mempool_id: MempoolID, page: usize) -> CornflakesSegment;

    /// Gets an iterator over 2MB pages in this mempool.
    fn get_2mb_pages(&self) -> Vec<usize>;

    fn get_4k_pages(&self) -> Vec<usize>;

    fn get_1g_pages(&self) -> Vec<usize>;

    /// Register the backing region behind this memory pool
    fn register_segment(
        &mut self,
        segment: &CornflakesSegment,
        registration_context: Self::RegistrationContext,
    ) -> Result<()>;

    /// Unregister the backing region behind this memory pool
    fn unregister_segment(&mut self, segment: &CornflakesSegment) -> Result<()>;

    /// Whether the entire backing region of the memory pool is registered
    fn is_registered(&self, segment: &CornflakesSegment) -> bool;

    /// Get backing pagesize.
    fn get_pagesize(&self) -> usize;

    fn has_allocated(&self) -> bool;

    fn recover_metadata(
        &self,
        buf: <<Self as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathBuffer,
    ) -> Result<<<Self as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathMetadata>;

    /// Recovers buffer into datapath metadata IF the buffer is registered and within bounds.
    /// MUST be called ONLY if the buffer is registered and within bounds.
    fn recover_buffer(
        &self,
        buf: &[u8],
    ) -> Result<<<Self as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathMetadata>;

    /// Allocate datapath buffer
    /// Given context to refer back to memory pool.
    fn alloc_data_buf(
        &self,
    ) -> Result<Option<<<Self as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathBuffer>>;
}

/// Datapaths can use this struct as their allocator, given an implementation of a
/// DatapathMemoryPool.
#[derive(Debug, PartialEq, Eq)]
pub struct MemoryPoolAllocator<M>
where
    M: DatapathMemoryPool + PartialEq + Eq + std::fmt::Debug,
{
    /// Map from pool size to list of currently allocated mempool IDs that applications can
    /// allocate memory from.
    mempool_ids: HashMap<usize, HashSet<MempoolID>>,

    /// HashMap of IDs to actual mempool object (receive mempools and those app can allocate
    /// from)
    mempools: HashMap<MempoolID, M>,

    /// Unique IDs assigned to mempools (assumes the number of mempools added won't overflow)
    next_id_to_allocate: MempoolID,

    /// Each core has one memory pool for allocating tx packets for copying data.
    /// (size, ID) pair.
    tx_mempool: M,

    /// Cache from allocated addresses -> corresponding metadata for 2mb pages.
    address_cache_2mb: AHashMap<usize, CornflakesSegment>,

    /// Cache from allocated address -> corresponding mempool ID for 4k pages.
    address_cache_4k: AHashMap<usize, CornflakesSegment>,

    /// Cache from allocated address -> corresponding mempool ID for 1G pages.
    address_cache_1g: AHashMap<usize, CornflakesSegment>,
}

impl<M> MemoryPoolAllocator<M>
where
    M: DatapathMemoryPool + PartialEq + Eq + std::fmt::Debug,
{
    pub fn new(rx_mempool: M, tx_mempool: M) -> Result<Self> {
        let mut address_cache_2mb = AHashMap::default();
        let mut address_cache_4k = AHashMap::default();
        let mut address_cache_1g = AHashMap::default();

        // add recv mempool to address cache
        for page in rx_mempool.get_2mb_pages().iter() {
            address_cache_2mb.insert(*page, rx_mempool.get_segment_info(0, *page));
        }
        for page in rx_mempool.get_4k_pages().iter() {
            address_cache_4k.insert(*page, rx_mempool.get_segment_info(0, *page));
        }
        for page in rx_mempool.get_1g_pages().iter() {
            address_cache_1g.insert(*page, rx_mempool.get_segment_info(0, *page));
        }

        let mut mempools = HashMap::default();
        mempools.insert(0, rx_mempool);

        tracing::info!("Address cache 2mb size: {}", address_cache_2mb.len());
        Ok(MemoryPoolAllocator {
            mempool_ids: HashMap::default(),
            mempools,
            next_id_to_allocate: 1,
            tx_mempool,
            address_cache_2mb,
            address_cache_4k,
            address_cache_1g,
        })
    }

    #[inline]
    fn find_cornflakes_segment(&self, buf: &[u8]) -> Option<CornflakesSegment> {
        match self
            .address_cache_2mb
            .get(&mem::closest_2mb_page(buf.as_ptr()))
        {
            Some(m) => {
                return Some(*m);
            }
            None => {}
        }
        match self
            .address_cache_4k
            .get(&mem::closest_4k_page(buf.as_ptr()))
        {
            Some(m) => {
                return Some(*m);
            }
            None => {}
        }
        match self
            .address_cache_1g
            .get(&mem::closest_1g_page(buf.as_ptr()))
        {
            Some(m) => {
                return Some(*m);
            }
            None => {}
        }
        return None;
    }

    #[inline]
    pub fn add_mempool(&mut self, size: usize, handle: M) -> Result<MempoolID> {
        tracing::debug!("Adding mempool");
        // add the mempool into the address cache
        for page in handle.get_2mb_pages().into_iter() {
            self.address_cache_2mb.insert(
                page,
                handle.get_segment_info(self.next_id_to_allocate, page),
            );
        }
        for page in handle.get_4k_pages().into_iter() {
            self.address_cache_4k.insert(
                page,
                handle.get_segment_info(self.next_id_to_allocate, page),
            );
        }
        for page in handle.get_1g_pages().into_iter() {
            self.address_cache_1g.insert(
                page,
                handle.get_segment_info(self.next_id_to_allocate, page),
            );
        }

        let aligned_size = align_to_pow2(size);
        tracing::info!("Adding mempool of size {}", aligned_size);
        match self.mempool_ids.get_mut(&aligned_size) {
            Some(mempool_ids_list) => {
                mempool_ids_list.insert(self.next_id_to_allocate);
                self.mempools.insert(self.next_id_to_allocate, handle);
                self.next_id_to_allocate += 1;
            }
            None => {
                let mut set = HashSet::default();
                set.insert(self.next_id_to_allocate);
                self.mempool_ids.insert(aligned_size, set);
                self.mempools.insert(self.next_id_to_allocate, handle);
                self.next_id_to_allocate += 1;
            }
        }
        Ok(self.next_id_to_allocate - 1)
    }

    /// Registers the given segment.
    /// If already registered, does nothing.
    #[inline]
    pub fn register_segment(
        &mut self,
        segment: &CornflakesSegment,
        context: M::RegistrationContext,
    ) -> Result<()> {
        match self.mempools.get_mut(&segment.get_mempool_id()) {
            Some(handle) => {
                handle.register_segment(segment, context)?;
            }
            None => {
                bail!(
                    "Mempool allocator does not contain mempool ID {:?}",
                    segment.get_mempool_id(),
                );
            }
        }
        Ok(())
    }

    /// Unregisters the given segment.
    /// If already unregistered, does nothing.
    #[inline]
    pub fn unregister_segment(&mut self, segment: &CornflakesSegment) -> Result<()> {
        match self.mempools.get_mut(&segment.get_mempool_id()) {
            Some(handle) => {
                handle.unregister_segment(segment)?;
            }
            None => {
                bail!(
                    "Mempool allocator does not contain mempool ID {:?}",
                    segment.get_mempool_id(),
                );
            }
        }
        Ok(())
    }

    #[inline]
    pub fn allocate_tx_buffer(
        &self,
    ) -> Result<Option<<<M as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathBuffer>> {
        tracing::debug!("Allocating from tx mempool: {:?}", self.tx_mempool);
        self.tx_mempool.alloc_data_buf()
    }

    /// Allocates a datapath buffer (if a mempool is available).
    /// Size will be aligned to the next power of 2. If no buffers available in that mempool, will
    /// return None.
    /// No guarantees on whether the resulting datapath buffer is registered or not.
    /// Adds buffer to address cache for possible recovery.
    #[inline]
    pub fn allocate_buffer(
        &mut self,
        buf_size: usize,
    ) -> Result<Option<<<M as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathBuffer>> {
        let align_size = align_to_pow2(buf_size);
        match self.mempool_ids.get(&align_size) {
            Some(mempools) => {
                for mempool_id in mempools.iter() {
                    let mempool = self.mempools.get(mempool_id).unwrap();
                    match mempool.alloc_data_buf()? {
                        Some(x) => {
                            return Ok(Some(x));
                        }
                        None => {}
                    }
                }
                return Ok(None);
            }
            None => {
                tracing::debug!("Returning none");
                return Ok(None);
            }
        }
    }

    /// Returns metadata associated with this buffer, if it exists, and the buffer is
    /// registered. Only looks at mempools with aligned size and receive mempools.
    #[inline]
    pub fn recover_buffer(
        &self,
        buffer: &[u8],
    ) -> Result<Option<<<M as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathMetadata>>
    {
        match self.find_cornflakes_segment(buffer) {
            Some(segment) => {
                let mempool = self.mempools.get(&segment.get_mempool_id()).unwrap();
                let metadata = mempool
                    .recover_buffer(buffer)
                    .wrap_err("unable to recover metadata")?;
                return Ok(Some(metadata));
            }
            None => {
                return Ok(None);
            }
        }
    }

    /// Returns metadata associated with this buffer, if it exists, and the buffer is
    /// registered. Only looks at mempools with aligned size and receive mempools.
    #[inline]
    pub fn recover_buffer_with_segment_info(
        &self,
        buffer: &[u8],
    ) -> Result<MetadataStatus<<M as DatapathMemoryPool>::DatapathImpl>> {
        match self.find_cornflakes_segment(buffer) {
            Some(segment) => {
                let mempool = self.mempools.get(&segment.get_mempool_id()).unwrap();
                let metadata = mempool
                    .recover_buffer(buffer)
                    .wrap_err("unable to recover metadata")?;
                if mempool.is_registered(&segment) {
                    // TODO: inefficient to return new CornflakesSegment.
                    // Perhaps this can eventually return &CornflakesSegment?
                    return Ok(MetadataStatus::Pinned((metadata, segment.clone())));
                } else {
                    return Ok(MetadataStatus::Pinned((metadata, segment.clone())));
                }
            }
            None => {
                return Ok(MetadataStatus::Arbitrary);
            }
        }
    }

    /// Checks whether a given datapath buffer is in registered memory or not.
    /// Checks only receive mempools and mempools with aligned to power of 2 size.
    #[inline]
    pub fn is_registered(&self, buf: &[u8]) -> bool {
        // search through recv mempools first
        match self.find_cornflakes_segment(buf) {
            Some(seg) => {
                let mempool = self.mempools.get(&seg.get_mempool_id()).unwrap();
                mempool.is_registered(&seg)
            }
            None => {
                return false;
            }
        }
    }

    #[inline]
    pub fn num_mempools_so_far(&self) -> u32 {
        self.next_id_to_allocate
    }

    #[inline]
    pub fn has_mempool(&self, size: usize) -> bool {
        return self.mempool_ids.contains_key(&size);
    }
}
