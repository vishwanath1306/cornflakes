use super::{
    super::{access, mlx5_bindings::*},
    connection::{MbufMetadata, Mlx5Buffer, Mlx5Connection, Mlx5PerThreadContext},
    sizes,
};
use color_eyre::eyre::{bail, Result};
use cornflakes_libos::{
    allocator::{DatapathMemoryPool, MempoolID},
    datapath::{CornflakesSegment, Datapath},
    mem::closest_2mb_page,
};
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
    ) -> (*mut ::std::os::raw::c_void, u64, usize, usize) {
        let data_pool = self.mempool();
        let mempool_start = access!(data_pool, buf, usize);
        let item_len = access!(data_pool, item_len, usize);
        let registration_unit = unsafe {
            custom_mlx5_mempool_find_registration_unit(
                data_pool,
                closest_2mb_page(ptr as *const u8) as *mut ::std::os::raw::c_void,
            )
        };
        let offset_within_alloc = ptr as usize - mempool_start;
        let index =
            (offset_within_alloc & !(item_len - 1)) >> access!(data_pool, log_item_len, usize);
        let data_ptr = (mempool_start + (index << access!(data_pool, log_item_len, usize)))
            as *mut std::os::raw::c_void;
        (
            data_ptr,
            registration_unit as _,
            index,
            ptr as usize - data_ptr as usize,
        )
    }
}

impl DatapathMemoryPool for DataMempool {
    type DatapathImpl = Mlx5Connection;

    type RegistrationContext = *mut custom_mlx5_per_thread_context;

    fn get_segment_info(&self, mempool_id: MempoolID, page: usize) -> CornflakesSegment {
        let mempool = self.mempool();
        let registration_unit = unsafe {
            custom_mlx5_mempool_find_registration_unit(mempool, page as *mut ::std::os::raw::c_void)
                as usize
        };
        CornflakesSegment::new(mempool_id, registration_unit, unsafe {
            access!(mempool, pgsize, usize)
        })
    }

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
    fn register_segment(
        &mut self,
        cornflakes_segment: &CornflakesSegment,
        registration_context: Self::RegistrationContext,
    ) -> Result<()> {
        unsafe {
            if custom_mlx5_register_mempool_unit(
                access!(
                    registration_context,
                    global_context,
                    *mut custom_mlx5_global_context
                ),
                self.mempool(),
                cornflakes_segment.get_registration_unit() as _,
                ibv_access_flags_IBV_ACCESS_LOCAL_WRITE as _,
            ) != 0
            {
                bail!("Failed to register mempool");
            }
        }
        Ok(())
    }

    #[inline]
    fn unregister_segment(&mut self, segment: &CornflakesSegment) -> Result<()> {
        unsafe {
            if custom_mlx5_deregister_mempool_unit(
                self.mempool(),
                segment.get_registration_unit() as _,
            ) != 0
            {
                bail!("Failed to deregister memory pool");
            }
        }
        Ok(())
    }

    #[inline]
    fn is_registered(&self, segment: &CornflakesSegment) -> bool {
        unsafe {
            custom_mlx5_is_registered(self.mempool(), segment.get_registration_unit() as _) == 1
        }
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
        let (data_ptr, registration_unit, index, offset) =
            unsafe { self.recover_metadata_mbuf(buf.as_ptr()) };

        {
            Ok(MbufMetadata::new(
                data_ptr,
                self.mempool(),
                registration_unit,
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
        let registration_unit = unsafe {
            custom_mlx5_mempool_find_registration_unit(
                self.mempool(),
                closest_2mb_page(data as *const u8) as *mut ::std::os::raw::c_void,
            )
        };
        if data.is_null() {
            tracing::debug!("Returning ok none ok");
            return Ok(None);
        }
        // recover the ref count index
        let index = unsafe { custom_mlx5_mempool_find_index(self.mempool(), data) };
        if index == -1 {
            unsafe {
                custom_mlx5_mempool_free(self.mempool(), data);
            }
            tracing::debug!("Couldn't find index");
            return Ok(None);
        }
        Ok(Some(Mlx5Buffer::new(
            data,
            self.mempool(),
            registration_unit,
            index as usize,
            0,
        )))
    }
}
