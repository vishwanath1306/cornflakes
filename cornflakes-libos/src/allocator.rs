use super::{datapath::Datapath, mem};
use ahash::AHashMap;
use color_eyre::eyre::{Result, WrapErr};
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

    /// Gets an iterator over 2MB pages in this mempool.
    fn get_2mb_pages(&self) -> Vec<usize>;

    fn get_4k_pages(&self) -> Vec<usize>;

    fn get_1g_pages(&self) -> Vec<usize>;

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
    address_cache_2mb: AHashMap<usize, MempoolID>,

    /// Cache from allocated address -> corresponding mempool ID for 4k pages.
    address_cache_4k: AHashMap<usize, MempoolID>,

    /// Cache from allocated address -> corresponding mempool ID for 1G pages.
    address_cache_1g: AHashMap<usize, MempoolID>,
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
            address_cache_2mb.insert(*page, 0);
        }
        for page in rx_mempool.get_4k_pages().iter() {
            address_cache_4k.insert(*page, 0);
        }
        for page in rx_mempool.get_1g_pages().iter() {
            address_cache_1g.insert(*page, 0);
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
    pub fn is_registered(&self, seg: &[u8]) -> bool {
        if let Some(_) = self.find_mempool_id(seg) {
            return true;
        }
        return false;
    }

    #[inline]
    fn find_mempool_id(&self, buf: &[u8]) -> Option<MempoolID> {
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
            self.address_cache_2mb
                .insert(page, self.next_id_to_allocate);
        }
        for page in handle.get_4k_pages().into_iter() {
            self.address_cache_4k.insert(page, self.next_id_to_allocate);
        }
        for page in handle.get_1g_pages().into_iter() {
            self.address_cache_1g.insert(page, self.next_id_to_allocate);
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

    /// Recovers metadata from this mempool ID.
    #[inline]
    pub fn recover_from_mempool(
        &self,
        mempool_id: MempoolID,
        buf: &[u8],
    ) -> Result<Option<<<M as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathMetadata>>
    {
        match self.mempools.get(&mempool_id) {
            Some(m) => {
                let metadata = m
                    .recover_buffer(buf)
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
    pub fn recover_buffer(
        &self,
        buffer: &[u8],
    ) -> Result<Option<<<M as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathMetadata>>
    {
        match self.find_mempool_id(buffer) {
            Some(mempool_id) => {
                let mempool = self.mempools.get(&mempool_id).unwrap();
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

    #[inline]
    pub fn num_mempools_so_far(&self) -> u32 {
        self.next_id_to_allocate
    }

    #[inline]
    pub fn has_mempool(&self, size: usize) -> bool {
        return self.mempool_ids.contains_key(&size);
    }
}
