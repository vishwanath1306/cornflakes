use super::{
    super::{access, mlx5_bindings::*},
    connection::{
        CornflakesMlx5Slab, MbufMetadata, Mlx5Buffer, Mlx5Connection, Mlx5PerThreadContext,
    },
    sizes,
};
use color_eyre::eyre::{bail, Result};
use cornflakes_libos::{allocator::DatapathMemoryPool, datapath::Datapath, mem::closest_2mb_page};
use std::boxed::Box;

#[derive(Debug, PartialEq, Eq)]
pub struct DataMempool {
    mempool_ptr: *mut [u8],
}

impl Drop for DataMempool {
    fn drop(&mut self) {
        // (a) drop pages behind mempool itself
        // (b) drop box allocated for registered mempool pointer
        unsafe {
            // TODO: might need to change drop to not try to unregister mempool
            if custom_mlx5_deregister_and_free_custom_mlx5_mempool(self.mempool()) != 0 {
                tracing::warn!(
                    "Failed to deregister and free backing mempool at {:?}",
                    self.mempool()
                );
            }
            tracing::warn!("Dropping data mempool {:?}", self.mempool_ptr);
            let _ = Box::from_raw(self.mempool_ptr);
        }
    }
}

impl DataMempool {
    #[inline]
    pub fn get_cornflakes_mlx5_slab(&self) -> Result<CornflakesMlx5Slab> {
        let page_size = match unsafe { access!(self.mempool(), pgsize, usize) } {
            zero_copy_cache::pagesizes::PGSIZE_4KB => zero_copy_cache::pagesizes::PageSize::PG4KB,
            zero_copy_cache::pagesizes::PGSIZE_2MB => zero_copy_cache::pagesizes::PageSize::PG2MB,
            zero_copy_cache::pagesizes::PGSIZE_1GB => zero_copy_cache::pagesizes::PageSize::PG1GB,
            _ => {
                bail!("Unacceptable pagesize: {}", unsafe {
                    access!(self.mempool(), pgsize, usize)
                });
            }
        };

        Ok(CornflakesMlx5Slab::new(
            0,
            page_size,
            unsafe { access!(self.mempool(), buf, *mut ::std::os::raw::c_void) },
            unsafe { access!(self.mempool(), num_pages, usize) },
        ))
    }

    #[inline]
    fn mempool(&self) -> *mut custom_mlx5_mempool {
        self.mempool_ptr as *mut custom_mlx5_mempool
    }

    #[inline]
    pub fn new_from_ptr(mempool_ptr: *mut [u8]) -> Self {
        tracing::info!("New mempool at ptr from ptr: {:?}", mempool_ptr,);
        DataMempool { mempool_ptr }
    }

    #[inline]
    pub fn new(
        mempool_params: &sizes::MempoolAllocationParams,
        per_thread_context: &Mlx5PerThreadContext,
        use_atomic_ops: bool,
        register_at_alloc: bool,
    ) -> Result<Self> {
        //println!("Allocating mempool with params: {:?}", mempool_params);
        let mempool_box = vec![0u8; unsafe { custom_mlx5_get_custom_mlx5_mempool_size() } as _]
            .into_boxed_slice();
        let atomic_ops: u32 = match use_atomic_ops {
            true => 1,
            false => 0,
        };
        let register: u32 = match register_at_alloc {
            true => 1,
            false => 0,
        };
        let mempool_ptr = Box::<[u8]>::into_raw(mempool_box);
        if unsafe {
            custom_mlx5_alloc_tx_pool(
                per_thread_context.get_context_ptr(),
                mempool_ptr as _,
                mempool_params.get_item_len() as _,
                mempool_params.get_num_items() as _,
                mempool_params.get_data_pgsize() as _,
                mempool_params.get_registration_unit() as _,
                ibv_access_flags_IBV_ACCESS_LOCAL_WRITE as _,
                atomic_ops,
                register,
            )
        } != 0
        {
            tracing::warn!(
                "Failed to register mempool with params {:?}",
                mempool_params
            );
            unsafe {
                let _ = Box::from_raw(mempool_ptr);
            }
            bail!("Failed register mempool with params {:?}", mempool_params);
        }
        tracing::info!("New mempool at ptr: {:?}", mempool_ptr,);
        Ok(DataMempool { mempool_ptr })
    }

    #[inline]
    pub unsafe fn recover_metadata_mbuf(
        &self,
        ptr: *const u8,
    ) -> (*mut ::std::os::raw::c_void, usize, usize) {
        let data_pool = self.mempool();
        let mempool_start = access!(data_pool, buf, usize);
        let item_len = access!(data_pool, item_len, usize);
        let offset_within_alloc = ptr as usize - mempool_start;
        let index =
            (offset_within_alloc & !(item_len - 1)) >> access!(data_pool, log_item_len, usize);
        let data_ptr = (mempool_start + (index << access!(data_pool, log_item_len, usize)))
            as *mut std::os::raw::c_void;
        (data_ptr, index, ptr as usize - data_ptr as usize)
    }
}

impl DatapathMemoryPool for DataMempool {
    type DatapathImpl = Mlx5Connection;

    type RegistrationContext = *mut custom_mlx5_per_thread_context;

    #[inline]
    fn get_2mb_pages(&self) -> Vec<usize> {
        let data_pool = self.mempool();
        let pgsize = unsafe { access!(data_pool, pgsize, usize) };
        if pgsize != cornflakes_libos::mem::PGSIZE_2MB {
            return vec![];
        }
        let num_pages = unsafe { access!(data_pool, num_pages, usize) };
        let mempool_start = unsafe { access!(data_pool, buf, usize) };
        (0..num_pages)
            .map(|i| mempool_start + pgsize * i)
            .collect::<Vec<usize>>()
    }

    #[inline]
    fn get_4k_pages(&self) -> Vec<usize> {
        let data_pool = self.mempool();
        let pgsize = unsafe { access!(data_pool, pgsize, usize) };
        if pgsize != cornflakes_libos::mem::PGSIZE_4KB {
            return vec![];
        }
        let num_pages = unsafe { access!(data_pool, num_pages, usize) };
        let mempool_start = unsafe { access!(data_pool, buf, usize) };
        (0..num_pages)
            .map(|i| mempool_start + pgsize * i)
            .collect::<Vec<usize>>()
    }

    #[inline]
    fn get_1g_pages(&self) -> Vec<usize> {
        let data_pool = self.mempool();
        let pgsize = unsafe { access!(data_pool, pgsize, usize) };
        if pgsize != cornflakes_libos::mem::PGSIZE_1GB {
            return vec![];
        }
        tracing::info!("In get 1g pages");
        let num_pages = unsafe { access!(data_pool, num_pages, usize) };
        let mempool_start = unsafe { access!(data_pool, buf, usize) };
        (0..num_pages)
            .map(|i| mempool_start + pgsize * i)
            .collect::<Vec<usize>>()
    }

    #[inline]
    fn get_pagesize(&self) -> usize {
        unsafe { access!(self.mempool(), pgsize, usize) }
    }

    #[inline(always)]
    fn has_allocated(&self) -> bool {
        unsafe { access!(self.mempool(), allocated, usize) >= 1 }
    }

    #[inline]
    fn recover_metadata(
        &self,
        buf: <<Self as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathBuffer,
    ) -> Result<<<Self as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathMetadata> {
        self.recover_buffer(buf.as_ref())
    }

    /// Recovers buffer into datapath metadata IF the buffer is registered and within bounds.
    /// MUST be called ONLY if the buffer is registered and within bounds.
    #[inline]
    fn recover_buffer(
        &self,
        buf: &[u8],
    ) -> Result<<<Self as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathMetadata> {
        let (data_ptr, index, offset) = unsafe { self.recover_metadata_mbuf(buf.as_ptr()) };
        {
            Ok(MbufMetadata::new(
                data_ptr,
                self.mempool(),
                0,
                index,
                offset,
                buf.len(),
            ))
        }
    }

    #[inline]
    fn alloc_data_buf(
        &self,
    ) -> Result<Option<<<Self as DatapathMemoryPool>::DatapathImpl as Datapath>::DatapathBuffer>>
    {
        let data = unsafe { custom_mlx5_mempool_alloc(self.mempool()) };
        if data.is_null() {
            return Ok(None);
        }
        let registration_unit = unsafe {
            custom_mlx5_mempool_find_registration_unit(
                self.mempool(),
                closest_2mb_page(data as *const u8) as *mut ::std::os::raw::c_void,
            )
        };
        let lkey = match unsafe { custom_mlx5_mempool_get_lkey(self.mempool(), registration_unit) }
        {
            -1 => None,
            x => {
                tracing::info!(
                    "Allocating data buf {:?} and adding lkey: {} from reg unit {}",
                    data,
                    x,
                    registration_unit
                );
                Some(x as u32)
            }
        };

        // recover the ref count index
        let index = unsafe { custom_mlx5_mempool_find_index(self.mempool(), data) };
        if index == -1 {
            unsafe {
                custom_mlx5_mempool_free(self.mempool(), data);
            }
            return Ok(None);
        }
        Ok(Some(Mlx5Buffer::new(
            data,
            self.mempool(),
            lkey,
            index as usize,
            0,
        )))
    }
}
