#pragma once

#include <infiniband/mlx5dv.h>
#include <infiniband/verbs.h>
#include <base/byteorder.h>
#include <base/mempool.h>

#define PORT_NUM 1

#define MAX_INLINE_DATA 256
#define RQ_NUM_DESC			1024
#define SQ_NUM_DESC			128

#define SQ_CLEAN_THRESH			1
#define SQ_CLEAN_MAX			SQ_CLEAN_THRESH
#define MAX_TX_MEMPOOLS_PER_THREAD 64 /* Maximum number of 'extra mempools' a thread can have. */
#define POW2MOD(num, size) ((num & (size - 1)))
#define current_segment(v) (POW2MOD(v->sq_head, v->tx_qp_dv.sq.wqe_cnt))

#define work_requests_size(v)(v->tx_qp_dv.sq.wqe_cnt * v->tx_qp_dv.sq.stride)
#define work_requests_start(v)((char *)v->tx_qp_dv.sq.buf)
#define work_requests_end(v)((char *)(work_requests_start(v) + work_requests_size(v)))
#define get_work_request(v, idx)((char *)(v->tx_qp_dv.sq.buf + (idx << v->tx_sq_log_stride)))

#define completion_buffer_start(v)((char *)(v->pending_transmissions[0]))
#define completion_buffer_end(v)(completion_buffer_start(v) + work_requests_size(v))
#define get_completion_segment(v, idx)((struct transmission_info *)(v->pending_transmissions[idx * 8]))

#define incr_ring_buffer(ptr, start, end, type, ptr_type) \
    ((((char *)ptr + sizeof(type)) == end) ? ((ptr_type)start) : ((ptr_type)((char *)ptr + (sizeof(type)))) )

/*
 * Direct hardware queue support
 */

struct __attribute__((__packed__)) transmission_info {
    union {
        struct __attribute__((__packed__)) transmission_metadata {
            uint32_t num_wqes; // number of descriptors used by this transmission
            uint32_t num_mbufs; // number of mbufs to decrease the reference count on
        } metadata;
        struct mbuf *mbuf;
    } info;
};


#define incr_dpseg(v, dpseg) (incr_ring_buffer(dpseg,  work_requests_start(v), work_requests_end(v), struct mlx5_wqe_data_seg, struct mlx5_wqe_data_seg *))

#define incr_transmission_info(v, transmission) (incr_ring_buffer(transmission, completion_buffer_start(v), completion_buffer_end(v), struct transmission_info, struct transmission_info *))

struct hardware_q {
	void		*descriptor_table;
	uint32_t	*consumer_idx;
	uint32_t	*shadow_tail;
	uint32_t	descriptor_log_size;
	uint32_t	nr_descriptors;
	uint32_t	parity_byte_offset;
	uint32_t	parity_bit_mask;
};

struct direct_txq {};

struct mlx5_rxq {
    /* handle for runtime */
	struct hardware_q rxq;

	uint32_t consumer_idx;

	struct mlx5dv_cq rx_cq_dv;
	struct mlx5dv_rwq rx_wq_dv;
	uint32_t wq_head;
	uint32_t rx_cq_log_stride;
	uint32_t rx_wq_log_stride;

	void **buffers; // array of posted buffers


	struct ibv_cq_ex *rx_cq;
	struct ibv_wq *rx_wq;
	struct ibv_rwq_ind_table *rwq_ind_table;
	struct ibv_qp *qp;

    size_t rx_hw_drop;
} __aligned(CACHE_LINE_SIZE);

struct mlx5_txq {
    /* handle for runtime */
	struct direct_txq txq;

	/* direct verbs qp */
	struct mbuf **buffers; // pending DMA
    struct transmission_info **pending_transmissions; // completion info for pending transmissions

	struct mlx5dv_qp tx_qp_dv;
	uint32_t sq_head;
	uint32_t tx_sq_log_stride;

	/* direct verbs cq */
	struct mlx5dv_cq tx_cq_dv;
	uint32_t cq_head;
    uint32_t true_cq_head;
	uint32_t tx_cq_log_stride;

	struct ibv_cq_ex *tx_cq;
	struct ibv_qp *tx_qp;
};

/* A registered memory pool. 
 * TODO: is it right for each mempool to have a unique registered / MR region.
 * Or can different `mempools` share the same backing registered region? */
struct registered_mempool {
    struct mempool data_mempool;
    struct mempool metadata_mempool;
    struct ibv_mr *mr; /* If this is null, this means the mempool isn't registered. */
    struct registered_mempool *next; /* Next allocated registered mempool in the list. */
};

inline void clear_registered_mempool(struct registered_mempool *mempool) {
    mempool->mr = NULL;
    mempool->next = NULL;
    clear_mempool(&mempool->data_mempool);
    clear_mempool(&mempool->metadata_mempool);
}

struct mlx5_global_context {
    struct ibv_context *ibv_context; /* IBV Context */
    struct ibv_pd *pd; /* pd variable */
    size_t num_threads; /* Number of total threads */
    struct mlx5_per_thread_context **thread_contexts; /* Per thread contexts */
    struct eth_addr *our_eth;
    struct ibv_rwq_ind_table *rwq_ind_table;
    struct ibv_qp *qp;
};

/* Per core information:
 * receive queue
 * send queue
 * rx metadata pool / data pool
 * */
struct mlx5_per_thread_context {
    size_t thread_id;
    struct mlx5_global_context *global_context; /* Pointer back to the global context. */
    struct mlx5_rxq rxq; /* Rxq for receiving packets. */
    struct mlx5_txq txq; /* Txq for sending packets. */
    struct registered_mempool rx_mempool; /* Receive mempool associated with the rxq. */
    struct mempool external_data_pool; /* Memory pool used for attaching external data.*/
    struct registered_mempool *tx_mempools;  /* Tx mempools linked list. */
    size_t num_allocated_tx_pools; /* Number of allocated tx pools. */
};

/* Given index into threads array, get per thread context. */
struct mlx5_per_thread_context *get_per_thread_context(struct mlx5_global_context *context, size_t idx);

/* Clears state in per thread context. */
inline void clear_per_thread_context(struct mlx5_global_context *context, size_t idx) {
    struct mlx5_per_thread_context *per_thread_context = get_per_thread_context(context, idx);
    per_thread_context->global_context = NULL;
    clear_registered_mempool(&per_thread_context->rx_mempool);
    clear_mempool(&per_thread_context->external_data_pool);
    per_thread_context->tx_mempools = NULL;
    per_thread_context->num_allocated_tx_pools = 0;
}

static inline unsigned int nr_inflight_tx(struct mlx5_txq *v)
{
	return v->sq_head - v->true_cq_head;
}

/*
 * cqe_status - retrieves status of completion queue element
 * @cqe: pointer to element
 * @cqe_cnt: total number of elements
 * @idx: index as stored in head pointer
 *
 * returns CQE status enum (MLX5_CQE_INVALID is -1)
 */
static inline uint8_t cqe_status(struct mlx5_cqe64 *cqe, uint32_t cqe_cnt, uint32_t head)
{
	uint16_t parity = head & cqe_cnt;
	uint8_t op_own = ACCESS_ONCE(cqe->op_own);
	uint8_t op_owner = op_own & MLX5_CQE_OWNER_MASK;
	uint8_t op_code = (op_own & 0xf0) >> 4;

	return ((op_owner == !parity) * MLX5_CQE_INVALID) | op_code;
}

static inline int mlx5_csum_ok(struct mlx5_cqe64 *cqe)
{
	return ((cqe->hds_ip_ext & (MLX5_CQE_L4_OK | MLX5_CQE_L3_OK)) ==
		 (MLX5_CQE_L4_OK | MLX5_CQE_L3_OK)) &
		(((cqe->l4_hdr_type_etc >> 2) & 0x3) == MLX5_CQE_L3_HDR_TYPE_IPV4);
}

static inline int mlx5_get_cqe_opcode(struct mlx5_cqe64 *cqe)
{
	return (cqe->op_own & 0xf0) >> 4;
}

static inline int mlx5_get_cqe_format(struct mlx5_cqe64 *cqe)
{
	return (cqe->op_own & 0xc) >> 2;
}

static inline int get_error_syndrome(struct mlx5_cqe64 *cqe) {
    return ((struct mlx5_err_cqe *)cqe)->syndrome;
}

static inline uint32_t mlx5_get_rss_result(struct mlx5_cqe64 *cqe)
{
	return ntoh32(*((uint32_t *)cqe + 3));
}