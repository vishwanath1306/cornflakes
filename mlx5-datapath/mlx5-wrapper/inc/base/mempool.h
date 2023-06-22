/*
 * mempool.h - a simple, preallocated pool of memory
 */

#pragma once

#include <base/stddef.h>
#include <base/debug.h>
#include <infiniband/mlx5dv.h>
#include <infiniband/verbs.h>

struct custom_mlx5_registration_info {
    int32_t lkey;
    struct ibv_mr *mr;
    void *starting_address;
};

struct custom_mlx5_mempool {
    void **free_items; /* Array of pointers to free items. */
    uint8_t *ref_counts; /* Array of reference counts for each item in the memory pool */
    size_t allocated; /* Number of allocated items. */
    size_t capacity; /* Total capacity of memory pool. */
    void *buf; /* Actual contiguous region of backing data. */
    size_t len; /* Total region length. */
    void *allocated_buf; /* actual allocated buf start for alignment purposes */
    size_t allocated_len; /* actual allocated length for aligmnemt purposes (= len + 4MB) */
    size_t pgsize; /* Page size. Using larger pages leads to TLB efficiency. */
    size_t item_len; /* Length of mempool items. Must be aligned to page size. */
    size_t log_item_len; /* Log of the item len*/
    size_t num_pages; /* Number of pages */
    size_t registration_len; /* Size of each registration unit */
    size_t nr_registrations; /* Number of registrations */
    struct custom_mlx5_registration_info *registrations; /* Array of registrations */
    uint32_t use_atomic_ops; /* Whether to use atomic operations to update the refcnt. */
};

int custom_mlx5_is_allocated(struct custom_mlx5_mempool *mempool);

void custom_mlx5_clear_mempool(struct custom_mlx5_mempool *mempool);

#ifdef DEBUG
extern void __custom_mlx5_mempool_alloc_debug_check(struct custom_mlx5_mempool *m, void *item);
extern void __custom_mlx5_mempool_free_debug_check(struct custom_mlx5_mempool *m, void *item);
#else /* DEBUG */
static inline void __custom_mlx5_mempool_alloc_debug_check(struct custom_mlx5_mempool *m, void *item) {}
static inline void __custom_mlx5_mempool_free_debug_check(struct custom_mlx5_mempool *m, void *item) {}
#endif /* DEBUG */

/*
 * custom_mlx5_mempool_is_registered - Checks if the given registration unit is
 * registered currently.
 */
int custom_mlx5_is_registered(struct custom_mlx5_mempool *mempool, size_t registration_unit);

/**
 * custom_mlx5_mempool_get_lkey - Gets the lkey for a given registration unit
 */
int32_t custom_mlx5_mempool_get_lkey(struct custom_mlx5_mempool *mempool, size_t registration_unit);
/**
 * custom_mlx5_mempool_find_registration_unit - Finds the registration unit
 * corresponding to a page start address.
 */
size_t custom_mlx5_mempool_find_registration_unit(struct custom_mlx5_mempool *mempool, void *page_address);


/**
 * mempool_find_index - Finds the allocated index of an item from the pool.
 * @m: the memory pool
 * @item: the object to find the index of.
 *
 * Returns index of item; -1 if item not within bounds of pool.
 */

int custom_mlx5_mempool_find_index(struct custom_mlx5_mempool *m, void *item);

/**
 * mempool_alloc - allocates an item from the pool
 * @m: the memory pool to allocate from
 *
 * Returns an item, or NULL if the pool is empty.
 */
void *custom_mlx5_mempool_alloc(struct custom_mlx5_mempool *m);

static inline void *custom_mlx5_mempool_alloc_by_idx(struct custom_mlx5_mempool *m, size_t idx)
{
    void *item;
    if (unlikely(m->allocated >= m->capacity)) {
        return NULL;
    }
    if (idx >= m->capacity) {
        return NULL;
    }
    item = m->free_items[idx];
    m->free_items[idx] = NULL;
    m->allocated++;
	__custom_mlx5_mempool_alloc_debug_check(m, item);
    return item;
}

/* 
 * mempool_free - returns an item to the pool
 * @m: the memory pool the item was allocated from
 * @item: the item to return
 * */
void custom_mlx5_mempool_free(struct custom_mlx5_mempool *m, void *item);

static inline void custom_mlx5_mempool_free_by_idx(struct custom_mlx5_mempool *m, void *item, size_t idx) {
	__custom_mlx5_mempool_free_debug_check(m, item);
    if (m->allocated == 0) {
        NETPERF_WARN("Freeing item %p item back into mempool %p with mem allocated 0.\n", m, item);
        return;
    }
    NETPERF_ASSERT(idx <= m->capacity, "Idx out of bounds");
    m->free_items[idx] = item;
    m->allocated--;
    NETPERF_ASSERT(m->allocated <= m->capacity, "Overflow in mempool"); /* Ensure no overflow */

}

int custom_mlx5_mempool_create(struct custom_mlx5_mempool *m,
                            size_t len,
                            size_t pgsize,
                            size_t item_len,
                            size_t registration_unit,
                            uint32_t use_atomic_ops);

void custom_mlx5_mempool_destroy(struct custom_mlx5_mempool *m);

