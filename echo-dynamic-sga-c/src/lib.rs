mod echo_messages_sga {
    include!(concat!(env!("OUT_DIR"), "/echo_dynamic_sga.rs"));
}

#[repr(C)]
pub struct SingleBufferCF {
    bitmap_len: usize,
    bitmap: *mut ::std::os::raw::c_char,
    message: *mut ::std::os::raw::c_char,
}

///////////////////////////////////////////////////////////////////////////////
// Generated functions in echo_dynamic_sga.rs

#[no_mangle]
pub extern "C" fn SingleBufferCF_new() -> *mut SingleBufferCF {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn SingleBufferCF_get_message(
    single_buffer_cf: *mut SingleBufferCF,
) -> *mut ::std::os::raw::c_char {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn SingleBufferCF_set_message(
    single_buffer_cf: *mut SingleBufferCF,
    message: *mut ::std::os::raw::c_char,
) {
    unimplemented!()
}

///////////////////////////////////////////////////////////////////////////////
// SgaHeaderRepr trait in echo_dynamic_sga.rs

#[no_mangle]
pub extern "C" fn SingleBufferCF_num_scatter_gather_entries(
    single_buffer_cf: *mut SingleBufferCF,
) -> usize {
    unimplemented!()
}

///////////////////////////////////////////////////////////////////////////////
// Shared functions in SgaHeaderRepr trait.
// See: cornflakes-codegen/src/utils/dynamic_sga_hdr.rs

#[no_mangle]
pub extern "C" fn SingleBufferCF_deserialize(
    single_buffer_cf: *mut SingleBufferCF,
    buffer: *mut ::std::os::raw::c_char,
) {
    unimplemented!()
}

#[no_mangle]
pub extern "C" fn SingleBufferCF_serialize_into_sga(
    single_buffer_cf: *mut SingleBufferCF,
    ordered_sga: *mut ::std::os::raw::c_void,
    conn: *mut ::std::os::raw::c_void,
) {
    unimplemented!()
}
