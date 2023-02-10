use color_eyre::eyre::{bail, Result};
use cornflakes_libos::mem;

pub const RX_MEMPOOL_DATA_PGSIZE: usize = mem::PGSIZE_2MB;
pub const RX_MEMPOOL_DATA_LEN: usize = 16384;
pub const RX_MEMPOOL_NUM_REGISTRATIONS: usize = 1;
pub const RX_MEMPOOL_MIN_NUM_ITEMS: usize = 8192;

pub fn align_up(x: usize, align_size: usize) -> usize {
    // find value aligned up to align_size
    let divisor = x / align_size;
    if (divisor * align_size) < x {
        return (divisor + 1) * align_size;
    } else {
        assert!(divisor * align_size == x);
        return x;
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct MempoolAllocationParams {
    item_len: usize,
    data_pgsize: usize,
    num_items: usize,
    num_data_pages: usize,
    num_registrations: usize,
}

impl MempoolAllocationParams {
    pub fn get_item_len(&self) -> usize {
        self.item_len
    }

    pub fn get_registration_unit(&self) -> usize {
        self.num_data_pages / self.num_registrations * self.data_pgsize
    }

    pub fn get_num_items(&self) -> usize {
        self.num_items
    }

    pub fn get_data_pgsize(&self) -> usize {
        self.data_pgsize
    }

    pub fn new(
        min_items: usize,
        data_pgsize: usize,
        item_size: usize,
        num_registrations: usize,
    ) -> Result<Self> {
        if data_pgsize != mem::PGSIZE_4KB
            && data_pgsize != mem::PGSIZE_2MB
            && data_pgsize != mem::PGSIZE_1GB
        {
            bail!("Data pgsize provided: {} not 4KB, 2MB, or 1GB", data_pgsize);
        }

        if data_pgsize % item_size != 0 {
            bail!(
                "Item size provided: {} not aligned to pgsize: {}",
                item_size,
                data_pgsize
            );
        }

        // calculate alignment number of objects
        let data_items_per_page = data_pgsize / item_size;

        // align the minimum number of objets up
        let num_items = align_up(min_items, data_items_per_page);

        // calculate the number of data pages and metadata pages accordingly
        let num_data_pages = num_items / data_items_per_page;

        if num_data_pages % num_registrations != 0 || num_data_pages < num_registrations {
            bail!("Mempool allocation params incorrect: cannot have {} registrations in mempool with {} pages", num_registrations, num_data_pages);
        }

        tracing::info!(
            min_items,
            data_items_per_page,
            num_data_pages,
            data_pgsize,
            item_size,
            num_items,
            num_registrations,
            "Final allocation params"
        );

        Ok(MempoolAllocationParams {
            item_len: item_size,
            data_pgsize,
            num_items,
            num_data_pages,
            num_registrations,
        })
    }
}
