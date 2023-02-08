/*
 * mlx5_init.h - initialization functions for datapath init and teardown.
 * */

#pragma once

#include <base/pci.h>
#include <base/debug.h>
#include <base/mempool.h>
#include <mlx5/mlx5.h>
#include <infiniband/verbs.h>
#include <infiniband/mlx5dv.h>

size_t custom_mlx5_get_global_context_size();

size_t custom_mlx5_get_custom_mlx5_mempool_size();

size_t custom_mlx5_get_per_thread_context_size(size_t num_threads);

void *custom_mlx5_get_raw_threads_ptr(struct custom_mlx5_global_context *global_context);

/* Allocates global context. */
void custom_mlx5_alloc_global_context(size_t num_threads, unsigned char *global_context_ptr, unsigned char *per_thread_info);

/* Attaches rx_mempool pointer to thread context. */
void custom_mlx5_set_rx_mempool_ptr(struct custom_mlx5_global_context *global_context,
                                        size_t thread_id,
                                        struct custom_mlx5_mempool *rx_mempool_ptr);

/* Allocate the data portions of the given memory pool. */
int custom_mlx5_allocate_mempool(struct custom_mlx5_mempool *mempool,
                        size_t item_len,
                        size_t num_items,
                        size_t data_pgsize,
                        size_t registration_unit_size,
                        uint32_t use_atomic_ops);

/* Create a data mempool, and register the data
 * mempool and store information in custom_mlx5_mempool object.*/
int custom_mlx5_create_mempool(struct custom_mlx5_global_context *context, 
                                    struct custom_mlx5_mempool *mempool,
                                    size_t item_len,
                                    size_t num_items,
                                    size_t data_pgsize,
                                    size_t registration_unit_size,
                                    int registry_flags,
                                    uint32_t use_atomic_ops,
                                    uint32_t register_at_alloc);

/* Registers a specific registration unit inside the memory pool.
 * Do nothing if it is already registered. */
int custom_mlx5_register_mempool_unit(struct custom_mlx5_global_context *context,
        struct custom_mlx5_mempool *mempool,
        size_t registration_unit,
        int flags);

/* Unregisters the region backing this memory pool. */
int custom_mlx5_deregister_mempool_unit(struct custom_mlx5_mempool *mempool,
        size_t registration_unit);

/* Unregisters region backing a memory pool, if necessary, and frees memory pool.*/
int custom_mlx5_deregister_and_free_custom_mlx5_mempool(struct custom_mlx5_mempool *mempool);

/* Initializes the rx mempools in each per thread context with the given params. */
int custom_mlx5_init_rx_mempools(struct custom_mlx5_global_context *context,
                        size_t item_len,
                        size_t num_items,
                        size_t data_pgsize,
                        int registry_flags);

/* Tears down rx mempool state until a certain thread id.*/
int custom_mlx5_free_rx_mempools(struct custom_mlx5_global_context *context, size_t max_idx);

/* Allocate pages for a new tx pool, given pointer to registered mempool data
 * structure */
int custom_mlx5_alloc_tx_pool(struct custom_mlx5_per_thread_context *t_context,
        struct custom_mlx5_mempool *mempool,
        size_t item_len,
        size_t num_items,
        size_t data_pgsize,
        size_t registration_unit_size,
        int registry_flags,
        uint32_t use_atomic_ops,
        uint32_t register_at_alloc);

/* Allocate and register a new tx mempool, given pointer to registered mempool. */
int custom_mlx5_alloc_and_register_tx_pool(struct custom_mlx5_per_thread_context *per_thread_context,
                                                        struct custom_mlx5_mempool *mempool,
                                                        size_t item_len, 
                                                        size_t num_items, 
                                                        size_t data_pgsize,
                                                        int registry_flags,
                                                        uint32_t use_atomic_ops);

/* Atomically read refcnt */
uint16_t custom_mlx5_refcnt_read(struct custom_mlx5_mempool *mempool,
        size_t refcnt_index);

/* Decrement reference count or return buffer to mempool. */
int custom_mlx5_refcnt_update_or_free(struct custom_mlx5_mempool *mempool, 
        void *buf, 
        size_t refcnt_index, 
        int8_t change);

/* Tearsdown state in the mlx5 per thread context. 
 * Includes:
 *  Freeing rx mempool
 * */
int custom_mlx5_teardown(struct custom_mlx5_per_thread_context *per_thread_context);

/* Helper function borrowed from DPDK. */
int custom_mlx5_ibv_device_to_pci_addr(const struct ibv_device *device, struct custom_mlx5_pci_addr *pci_addr);

/* Initializes ibv context within global context. */
int custom_mlx5_init_ibv_context(struct custom_mlx5_global_context *global_context,
                        struct custom_mlx5_pci_addr *nic_pci_addr);

/* Queue steering initialization for rxqs within the global context. 
 * Requires rxqs for each thread to be initialized / allocated. */
int custom_mlx5_qs_init_flows(struct custom_mlx5_global_context *global_context, struct eth_addr *our_eth);

/* Individual rxq initialization. */
int custom_mlx5_init_rxq(struct custom_mlx5_per_thread_context *thread_context);

/* Individual txq initialization. */
int custom_mlx5_init_txq(struct custom_mlx5_per_thread_context *thread_context);

