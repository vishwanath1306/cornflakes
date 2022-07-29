use super::HeaderType;
use ffiber::{CDylibCompiler, types::*, compiler::*};
use color_eyre::eyre::Result;

pub fn compile(output_folder: &str, header_type: HeaderType) -> Result<()> {
    let mut compiler = CDylibCompiler::new("mlx5-datapath", &output_folder);
    gen_cargo_toml(&mut compiler)?;
    gen_dependencies(&mut compiler, header_type)?;
    gen_received_pkt(&mut compiler)?;
    gen_constructor(&mut compiler)?;
    gen_configuration(&mut compiler)?;
    gen_pop(&mut compiler)?;
    gen_push(&mut compiler, header_type)?;
    compiler.flush()?;
    Ok(())
}

fn gen_cargo_toml(compiler: &mut CDylibCompiler) -> Result<()> {
    compiler.add_crate_version("color-eyre", "0.5");
    compiler.add_crate("cornflakes-libos",
        vec![("path", "\"../../../cornflakes-libos\"")])?;
    compiler.add_crate("cornflakes-utils",
        vec![("path", "\"../../../cornflakes-utils\"")])?;
    compiler.add_crate("mlx5-datapath",
        vec![("path", "\"../../../mlx5-datapath\"")])?;
    Ok(())
}

fn gen_dependencies(
    compiler: &mut CDylibCompiler,
    header_type: HeaderType,
) -> Result<()> {
    compiler.add_dependency(match header_type {
        HeaderType::Sga => "cornflakes_libos::OrderedSga",
        HeaderType::RcSga => "cornflakes_libos::ArenaOrderedRcSga",
        _ => unimplemented!(),
    })?;
    compiler.add_dependency("cornflakes_libos::datapath::{Datapath, InlineMode, ReceivedPkt}")?;
    compiler.add_dependency("cornflakes_utils::AppMode")?;
    compiler.add_dependency("color_eyre::eyre::Result")?;
    compiler.add_dependency("mlx5_datapath::datapath::connection::Mlx5Connection")?;
    compiler.add_dependency("std::{ffi::CStr, net::Ipv4Addr, str::FromStr}")?;
    Ok(())
}

fn gen_received_pkt(compiler: &mut CDylibCompiler) -> Result<()> {
    let struct_ty = ArgType::Struct {
        name: "ReceivedPkt".to_string(),
        params: vec![Box::new(ArgType::new_struct("Mlx5Connection"))],
    };

    ////////////////////////////////////////////////////////////////////////////
    // ReceivedPkt_msg_id
    compiler.add_extern_c_function(
        struct_ty.clone(),
        SelfArgType::Value,
        "msg_id",
        vec![],
        Some(ArgType::Primitive("u32".to_string())),
        false,
    )?;

    ////////////////////////////////////////////////////////////////////////////
    // ReceivedPkt_conn_id
    compiler.add_extern_c_function(
        struct_ty.clone(),
        SelfArgType::Value,
        "conn_id",
        vec![],
        Some(ArgType::Primitive("usize".to_string())),
        false,
    )?;

    ////////////////////////////////////////////////////////////////////////////
    // ReceivedPkt_unpack
    let args = vec![
        FunctionArg::CArg(CArgInfo::arg("self_", "*const ::std::os::raw::c_void")),
        FunctionArg::CArg(CArgInfo::ret_arg("*const ::std::os::raw::c_uchar")),
        FunctionArg::CArg(CArgInfo::ret_len_arg()),
    ];
    let func_context = FunctionContext::new_extern_c(
        "ReceivedPkt_unpack", true, args, false,
    );
    compiler.inner.add_context(Context::Function(func_context))?;
    compiler.inner.add_unsafe_def_with_let(false, None, "self_",
        &format!("Box::from_raw(self_ as *mut {})", &struct_ty.to_rust_str()))?;
    compiler.inner.add_def_with_let(false, None, "pkt", "self_.seg(0)")?;
    compiler.inner.add_unsafe_set("return_ptr", "pkt.as_ref().as_ptr()")?;
    compiler.inner.add_unsafe_set("return_len_ptr", "pkt.as_ref().len()")?;
    compiler.inner.add_func_call(None, "Box::into_raw",
        vec!["self_".to_string()], false)?;
    compiler.inner.pop_context()?; // end of function
    compiler.inner.add_newline()?;
    Ok(())
}

fn gen_constructor(compiler: &mut CDylibCompiler) -> Result<()> {
    ////////////////////////////////////////////////////////////////////////////
    // convert_c_char
    let args = vec![
        FunctionArg::CArg(CArgInfo::arg("ptr", "*const ::std::os::raw::c_char")),
    ];
    compiler.inner.add_context(Context::Function(FunctionContext::new(
        "convert_c_char", false, args, "String",
    )))?;
    compiler.inner.add_unsafe_def_with_let(false, Some("&CStr".to_string()), "cstr",
        "CStr::from_ptr(ptr)")?;
    compiler.inner.add_def_with_let(false, Some("&str".to_string()), "str_slice",
        "cstr.to_str().unwrap()")?;
    compiler.inner.add_line("str_slice.to_string()")?;
    compiler.inner.pop_context()?; // end of function
    compiler.inner.add_newline()?;

    ////////////////////////////////////////////////////////////////////////////
    // Mlx5Connection_new
    let args = vec![
        FunctionArg::CArg(CArgInfo::arg("config_file", "*const ::std::os::raw::c_char")),
        FunctionArg::CArg(CArgInfo::arg("server_ip", "*const ::std::os::raw::c_char")),
        FunctionArg::CArg(CArgInfo::ret_arg("*mut ::std::os::raw::c_void")),
    ];
    compiler.inner.add_context(Context::Function(FunctionContext::new_extern_c(
        "Mlx5Connection_new", true, args, true,
    )))?;

    compiler.inner.add_context(Context::Match(MatchContext::new_with_def(
        "Mlx5Connection::parse_config_file( \
            convert_c_char(config_file).as_str(), \
            &Ipv4Addr::from_str(convert_c_char(server_ip).as_str()).unwrap(), \
        )",
        vec!["Ok(value)".to_string(), "Err(_)".to_string()],
        "mut datapath_params",
    )))?;
    compiler.inner.add_return_val("value", false)?;
    compiler.inner.pop_context()?;
    compiler.inner.add_return_val("1", true)?;
    compiler.inner.pop_context()?;

    compiler.inner.add_context(Context::Match(MatchContext::new_with_def(
        "Mlx5Connection::compute_affinity(&datapath_params, 1, None, AppMode::Server)",
        vec!["Ok(value)".to_string(), "Err(_)".to_string()],
        "addresses",
    )))?;
    compiler.inner.add_return_val("value", false)?;
    compiler.inner.pop_context()?;
    compiler.inner.add_return_val("1", true)?;
    compiler.inner.pop_context()?;

    compiler.inner.add_context(Context::Match(MatchContext::new_with_def(
        "Mlx5Connection::global_init(1, &mut datapath_params, addresses)",
        vec!["Ok(value)".to_string(), "Err(_)".to_string()],
        "per_thread_contexts",
    )))?;
    compiler.inner.add_return_val("value", false)?;
    compiler.inner.pop_context()?;
    compiler.inner.add_return_val("1", true)?;
    compiler.inner.pop_context()?;

    compiler.inner.add_context(Context::Match(MatchContext::new_with_def(
        "Mlx5Connection::per_thread_init( \
            datapath_params, \
            per_thread_contexts.into_iter().nth(0).unwrap(), \
            AppMode::Server, \
        )",
        vec!["Ok(value)".to_string(), "Err(_)".to_string()],
        "conn",
    )))?;
    compiler.inner.add_return_val("value", false)?;
    compiler.inner.pop_context()?;
    compiler.inner.add_return_val("1", true)?;
    compiler.inner.pop_context()?;

    compiler.inner.add_def_with_let(false, None, "conn_box",
        "Box::into_raw(Box::new(conn))")?;
    compiler.inner.add_unsafe_set("return_ptr", "conn_box as _")?;
    compiler.inner.add_return_val("0", false)?;
    compiler.inner.pop_context()?; // end of function
    compiler.inner.add_newline()?;

    Ok(())
}

fn gen_configuration(compiler: &mut CDylibCompiler) -> Result<()> {
    ////////////////////////////////////////////////////////////////////////////
    // Mlx5Connection_set_copying_threshold
    compiler.add_extern_c_function(
        ArgType::new_struct("Mlx5Connection"),
        SelfArgType::Mut,
        "set_copying_threshold",
        vec![
            ("copying_threshold", ArgType::Primitive("usize".to_string())),
        ],
        None,
        false,
    )?;

    ////////////////////////////////////////////////////////////////////////////
    // Mlx5Connection_set_inline_mode
    compiler.add_extern_c_function(
        ArgType::new_struct("Mlx5Connection"),
        SelfArgType::Mut,
        "set_inline_mode",
        vec![
            ("inline_mode", ArgType::Enum {
                name: "InlineMode".to_string(),
                variants: vec![
                    "Nothing".to_string(),
                    "PacketHeader".to_string(),
                    "ObjectHeader".to_string(),
                ],
            }),
        ],
        None,
        false,
    )?;

    ////////////////////////////////////////////////////////////////////////////
    // Mlx5Connection_add_memory_pool
    compiler.add_extern_c_function(
        ArgType::new_struct("Mlx5Connection"),
        SelfArgType::Mut,
        "add_memory_pool",
        vec![
            ("buf_size", ArgType::Primitive("usize".to_string())),
            ("min_elts", ArgType::Primitive("usize".to_string())),
        ],
        None,
        true,
    )?;

    ////////////////////////////////////////////////////////////////////////////
    // Mlx5Connection_add_tx_mempool
    compiler.add_extern_c_function(
        ArgType::new_struct("Mlx5Connection"),
        SelfArgType::Mut,
        "add_tx_mempool",
        vec![
            ("size", ArgType::Primitive("usize".to_string())),
            ("min_elts", ArgType::Primitive("usize".to_string())),
        ],
        None,
        true,
    )?;
    Ok(())
}

fn gen_pop(compiler: &mut CDylibCompiler) -> Result<()> {
    ////////////////////////////////////////////////////////////////////////////
    // Mlx5Connection_pop
    compiler.add_extern_c_function(
        ArgType::new_struct("Mlx5Connection"),
        SelfArgType::Mut,
        "pop",
        vec![],
        Some(ArgType::Buffer(Box::new(ArgType::Struct {
            name: "ReceivedPkt".to_string(),
            params: vec![Box::new(ArgType::new_struct("Mlx5Connection"))],
        }))),
        true,
    )?;
    Ok(())
}

fn gen_push(
    compiler: &mut CDylibCompiler,
    header_type: HeaderType,
) -> Result<()> {
    match header_type {
        HeaderType::Sga => { add_push_ordered_sgas_fn(compiler)?; }
        HeaderType::RcSga => { add_queue_arena_ordered_sga_fn(compiler)?; }
        _ => { unimplemented!() }
    }

    ////////////////////////////////////////////////////////////////////////////
    // Mlx5Connection_queue_single_buffer_with_copy
    let args = vec![
        FunctionArg::CArg(CArgInfo::arg("conn", "&mut Mlx5Connection")),
        FunctionArg::CArg(CArgInfo::arg("msg_id", "u32")),
        FunctionArg::CArg(CArgInfo::arg("conn_id", "usize")),
        FunctionArg::CArg(CArgInfo::arg("buffer", "&[u8]")),
        FunctionArg::CArg(CArgInfo::arg("end_batch", "bool")),
    ];
    compiler.inner.add_context(Context::Function(FunctionContext::new(
        "Mlx5Connection_queue_single_buffer_with_copy_inner",
        false, args, "Result<()>",
    )))?;
    compiler.inner.add_line("conn.queue_single_buffer_with_copy((msg_id, conn_id, buffer), end_batch)")?;
    compiler.inner.pop_context()?; // end of function
    compiler.inner.add_newline()?;
    compiler.add_extern_c_function_standalone(
        "Mlx5Connection_queue_single_buffer_with_copy",
        "Mlx5Connection_queue_single_buffer_with_copy_inner",
        vec![
            ("conn", ArgType::new_ref_mut("Mlx5Connection")),
            ("msg_id", ArgType::Primitive("u32".to_string())),
            ("conn_id", ArgType::Primitive("usize".to_string())),
            ("buffer", ArgType::new_u8_buffer()),
            ("end_batch", ArgType::Primitive("bool".to_string())),
        ],
        None,
        true,
    )?;
    Ok(())
}

fn add_push_ordered_sgas_fn(compiler: &mut CDylibCompiler) -> Result<()> {
    let args = vec![
        FunctionArg::CArg(CArgInfo::arg("conn", "&mut Mlx5Connection")),
        FunctionArg::CArg(CArgInfo::arg("n", "usize")),
        FunctionArg::CArg(CArgInfo::arg("msg_ids", "&[u32]")),
        FunctionArg::CArg(CArgInfo::arg("conn_ids", "&[usize]")),
        FunctionArg::CArg(CArgInfo::arg("sgas", "&[*mut OrderedSga]")),
    ];
    compiler.inner.add_context(Context::Function(FunctionContext::new(
        "Mlx5Connection_push_ordered_sgas_inner",
        false, args, "Result<()>",
    )))?;
    compiler.inner.add_def_with_let(
        false,
        Some("Vec<_>".to_string()),
        "ordered_sgas",
        "(0..n)
            .map(|i| (msg_ids[i], conn_ids[i], unsafe { *Box::from_raw(sgas[i]) }))
            .collect()",
    )?;
    compiler.inner.add_line("conn.push_ordered_sgas(&ordered_sgas[..])")?;
    compiler.inner.pop_context()?; // end of function
    compiler.inner.add_newline()?;
    compiler.add_extern_c_function_standalone(
        "Mlx5Connection_push_ordered_sgas",
        "Mlx5Connection_push_ordered_sgas_inner",
        vec![
            ("conn", ArgType::new_ref_mut("Mlx5Connection")),
            ("n", ArgType::Primitive("usize".to_string())),
            ("msg_ids", ArgType::Buffer(Box::new(ArgType::Primitive("u32".to_string())))),
            ("conn_ids", ArgType::Buffer(Box::new(ArgType::Primitive("usize".to_string())))),
            ("sgas", ArgType::Buffer(Box::new(ArgType::RefMut(Box::new(
                ArgType::new_struct("OrderedSga")))))),
        ],
        None,
        true,
    )?;
    Ok(())
}

fn add_queue_arena_ordered_sga_fn(compiler: &mut CDylibCompiler) -> Result<()> {
    let args = vec![
        FunctionArg::CArg(CArgInfo::arg("conn", "&mut Mlx5Connection")),
        FunctionArg::CArg(CArgInfo::arg("msg_id", "u32")),
        FunctionArg::CArg(CArgInfo::arg("conn_id", "usize")),
        FunctionArg::CArg(CArgInfo::arg("rcsga", "ArenaOrderedRcSga<Mlx5Connection>")),
        FunctionArg::CArg(CArgInfo::arg("end_batch", "bool")),
    ];
    compiler.inner.add_context(Context::Function(FunctionContext::new(
        "Mlx5Connection_queue_arena_ordered_rcsga_inner",
        false, args, "Result<()>",
    )))?;
    compiler.inner.add_line("conn.queue_arena_ordered_rcsga((msg_id, conn_id, rcsga), end_batch)")?;
    compiler.inner.pop_context()?; // end of function
    compiler.inner.add_newline()?;
    compiler.add_extern_c_function_standalone(
        "Mlx5Connection_queue_arena_ordered_rcsga",
        "Mlx5Connection_queue_arena_ordered_rcsga_inner",
        vec![
            ("conn", ArgType::new_ref_mut("Mlx5Connection")),
            ("msg_id", ArgType::Primitive("u32".to_string())),
            ("conn_id", ArgType::Primitive("usize".to_string())),
            ("rcsga", ArgType::Struct {
                name: "ArenaOrderedRcSga".to_string(),
                params: vec![Box::new(ArgType::new_struct("Mlx5Connection"))],
            }),
            ("end_batch", ArgType::Primitive("bool".to_string())),
        ],
        None,
        true,
    )?;
    Ok(())
}
