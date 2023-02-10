#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(improper_ctypes)]

include!(concat!(env!("OUT_DIR"), "/mlx5_bindings.rs"));

#[link(name = "mlx5inlined")]
extern "C" {
    fn ns_to_cycles_(a: u64) -> u64;

    fn cycles_to_ns_(a: u64) -> u64;

    fn current_cycles_() -> u64;

    fn alloc_data_buf_(mempool: *mut custom_mlx5_mempool) -> *mut ::std::os::raw::c_void;

    fn mlx5_rte_memcpy_(
        dst: *mut ::std::os::raw::c_void,
        src: *const ::std::os::raw::c_void,
        n: usize,
    );

    fn custom_mlx5_fill_in_hdrs_(
        hdr_buffer: *mut ::std::os::raw::c_void,
        hdr: *const ::std::os::raw::c_void,
        id: u32,
        data_len: usize,
    );

    fn custom_mlx5_completion_start_(
        context: *mut custom_mlx5_per_thread_context,
    ) -> *mut custom_mlx5_transmission_info;

    fn custom_mlx5_dpseg_start_(
        context: *mut custom_mlx5_per_thread_context,
        inline_off: usize,
    ) -> *mut mlx5_wqe_data_seg;

    fn flip_headers_mlx5_(data: *mut ::std::os::raw::c_void);

}

#[inline]
pub unsafe fn ns_to_cycles(a: u64) -> u64 {
    ns_to_cycles_(a)
}

#[inline]
pub unsafe fn cycles_to_ns(a: u64) -> u64 {
    cycles_to_ns_(a)
}

#[inline]
pub unsafe fn current_cycles() -> u64 {
    current_cycles_()
}

#[inline]
pub unsafe fn alloc_data_buf(mempool: *mut custom_mlx5_mempool) -> *mut ::std::os::raw::c_void {
    alloc_data_buf_(mempool)
}

#[inline]
pub unsafe fn mlx5_rte_memcpy(
    dst: *mut ::std::os::raw::c_void,
    src: *const ::std::os::raw::c_void,
    n: usize,
) {
    mlx5_rte_memcpy_(dst, src, n);
}

#[inline]
pub unsafe fn fill_in_hdrs(
    hdr_buffer: *mut ::std::os::raw::c_void,
    hdr: *const ::std::os::raw::c_void,
    id: u32,
    data_len: usize,
) {
    custom_mlx5_fill_in_hdrs_(hdr_buffer, hdr, id, data_len);
}

#[inline]
pub unsafe fn custom_mlx5_completion_start(
    context: *mut custom_mlx5_per_thread_context,
) -> *mut custom_mlx5_transmission_info {
    custom_mlx5_completion_start_(context)
}

#[inline]
pub unsafe fn custom_mlx5_dpseg_start(
    context: *mut custom_mlx5_per_thread_context,
    inline_off: usize,
) -> *mut mlx5_wqe_data_seg {
    custom_mlx5_dpseg_start_(context, inline_off)
}

#[inline]
pub unsafe fn flip_headers(data: *mut ::std::os::raw::c_void) {
    flip_headers_mlx5_(data);
}
