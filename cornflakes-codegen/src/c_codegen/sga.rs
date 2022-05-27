use super::{
    super::header_utils::{FieldInfo, MessageInfo, ProtoReprInfo},
    super::rust_codegen::{
        ArgInfo, Context, FunctionArg, FunctionContext, ImplContext, LoopBranch,
        LoopContext, SerializationCompiler, StructContext, StructDefContext,
        StructName, TraitName,
    },
};
use color_eyre::eyre::{bail, Result};
use protobuf_parser::FieldType;

pub fn compile(fd: &ProtoReprInfo, compiler: &mut SerializationCompiler) -> Result<()> {
    // add_dependencies(fd, compiler)?;
    for message in fd.get_repr().messages.iter() {
        // compiler.add_newline()?;
        // let msg_info = MessageInfo(message.clone());
        // add_struct_definition(fd, compiler, &msg_info)?;
        // compiler.add_newline()?;
        // add_default_impl(fd, compiler, &msg_info)?;
        // compiler.add_newline()?;
        // add_impl(fd, compiler, &msg_info)?;
        // compiler.add_newline()?;
        // add_header_repr(fd, compiler, &msg_info)?;
    }
    Ok(())
}

fn add_dependencies(repr: &ProtoReprInfo, compiler: &mut SerializationCompiler) -> Result<()> {
    compiler.add_dependency("cornflakes_libos::Sge")?;
    compiler.add_dependency("color_eyre::eyre::Result")?;
    compiler.add_dependency("cornflakes_codegen::utils::dynamic_sga_hdr::*")?;
    compiler.add_dependency("cornflakes_codegen::utils::dynamic_sga_hdr::{SgaHeaderRepr}")?;

    // if any message has integers, we need slice
    if repr.has_int_field() {
        compiler.add_dependency("std::{slice}")?;
    }
    Ok(())
}

fn add_struct_definition(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
) -> Result<()> {
    // no lifetime needed
    let type_annotations = msg_info.get_type_params(false, &fd)?;
    let where_clause = msg_info.get_where_clause(false, &fd)?;
    let struct_name = StructName::new(&msg_info.get_name(), type_annotations.clone());
    let struct_ctx = StructContext::new(
        struct_name,
        msg_info.derives_copy(&fd.get_message_map(), true)?,
        where_clause,
    );

    // add struct header
    compiler.add_context(Context::Struct(struct_ctx))?;
    compiler.add_struct_field("bitmap", &format!("Vec<u8>"))?;
    // write in struct fields
    for field in msg_info.get_fields().iter() {
        let field_info = FieldInfo(field.clone());
        compiler.add_struct_field(&field_info.get_name(), &fd.get_rust_type(field_info)?)?;
    }
    compiler.pop_context()?;
    Ok(())
}

fn add_default_impl(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
) -> Result<()> {
    let type_annotations = msg_info.get_type_params(false, &fd)?;
    let where_clause = msg_info.get_where_clause(false, &fd)?;
    let struct_name = StructName::new(&msg_info.get_name(), type_annotations.clone());
    let trait_name = TraitName::new("Default", vec![]);
    let impl_context = ImplContext::new(struct_name, Some(trait_name), where_clause);
    compiler.add_context(Context::Impl(impl_context))?;
    let func_context = FunctionContext::new("default", false, Vec::default(), "Self");
    compiler.add_context(Context::Function(func_context))?;
    let struct_def_context = StructDefContext::new(&msg_info.get_name());
    compiler.add_context(Context::StructDef(struct_def_context))?;
    compiler.add_struct_def_field("bitmap", &format!("vec![0u8; Self::bitmap_length()]"))?;
    for field in msg_info.get_fields().iter() {
        let field_info = FieldInfo(field.clone());
        if field_info.is_list() {
            if field_info.is_int_list() {
                compiler.add_struct_def_field(&field_info.get_name(), "List::default()")?;
            } else {
                compiler.add_struct_def_field(&field_info.get_name(), "VariableList::default()")?;
            }
        } else {
            compiler
                .add_struct_def_field(&field_info.get_name(), &fd.get_default_type(field_info)?)?;
        }
    }
    compiler.pop_context()?; // end of struct definition
    compiler.pop_context()?; // end of function
    compiler.pop_context()?;
    Ok(())
}

fn add_impl(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
) -> Result<()> {
    let type_annotations = msg_info.get_type_params(false, &fd)?;
    let where_clause = msg_info.get_where_clause(false, &fd)?;
    let impl_context = ImplContext::new(
        StructName::new(&msg_info.get_name(), type_annotations.clone()),
        None,
        where_clause,
    );

    compiler.add_context(Context::Impl(impl_context))?;
    for field in msg_info.get_fields().iter() {
        compiler.add_newline()?;
        let field_info = FieldInfo(field.clone());
        for (var, typ, def) in msg_info.get_constants(&field_info, false, false)?.iter() {
            compiler.add_const_def(var, typ, def)?;
        }
    }

    compiler.add_newline()?;
    let new_func_context = FunctionContext::new("new", true, vec![], "Self");
    compiler.add_context(Context::Function(new_func_context))?;
    let struct_def_context = StructDefContext::new(&msg_info.get_name());
    compiler.add_context(Context::StructDef(struct_def_context))?;
    compiler.add_struct_def_field("bitmap", "vec![0u8; Self::bitmap_length()]")?;
    for field in msg_info.get_fields().iter() {
        let field_info = FieldInfo(field.clone());
        if field_info.is_list() {
            if field_info.is_int_list() {
                compiler.add_struct_def_field(&field_info.get_name(), "List::default()")?;
            } else {
                compiler.add_struct_def_field(&field_info.get_name(), "VariableList::default()")?;
            }
        } else {
            compiler
                .add_struct_def_field(&field_info.get_name(), &fd.get_default_type(field_info)?)?;
        }
    }

    compiler.pop_context()?; // end of struct definition
    compiler.pop_context()?; // end of new function

    for field in msg_info.get_fields().iter() {
        let field_info = FieldInfo(field.clone());
        add_field_methods(fd, compiler, &field_info)?;
    }

    compiler.pop_context()?;
    Ok(())
}

fn add_field_methods(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    field: &FieldInfo,
) -> Result<()> {
    // add has_x, get_x, set_x
    compiler.add_newline()?;
    add_has(compiler, field)?;
    compiler.add_newline()?;
    add_get(fd, compiler, field)?;
    compiler.add_newline()?;
    add_set(fd, compiler, field)?;
    compiler.add_newline()?;

    // if field is a list or a nested struct, add get_mut_x
    if field.is_list() || field.is_nested_msg() {
        add_get_mut(fd, compiler, field)?;
    }

    // if field is list, add init_x
    if field.is_list() {
        add_list_init(compiler, field)?;
    }
    Ok(())
}

fn add_has(compiler: &mut SerializationCompiler, field: &FieldInfo) -> Result<()> {
    let func_context = FunctionContext::new(
        &format!("has_{}", field.get_name()),
        true,
        vec![FunctionArg::SelfArg],
        "bool",
    );
    compiler.add_context(Context::Function(func_context))?;
    let bitmap_field_str = field.get_bitmap_idx_str(true);
    compiler.add_return_val(
        &format!("self.get_bitmap_field({})", bitmap_field_str),
        false,
    )?;
    compiler.pop_context()?;
    Ok(())
}

fn add_get(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    field: &FieldInfo,
) -> Result<()> {
    let field_type = fd.get_rust_type(field.clone())?;
    let return_type = match field.is_list() || field.is_nested_msg() {
        true => format!("&{}", field_type),
        false => field_type.to_string(),
    };
    let func_context = FunctionContext::new(
        &format!("get_{}", field.get_name()),
        true,
        vec![FunctionArg::SelfArg],
        &return_type,
    );
    compiler.add_context(Context::Function(func_context))?;
    let return_val = match field.is_list() || field.is_nested_msg() {
        true => format!("&self.{}", field.get_name()),
        false => format!("self.{}", field.get_name()),
    };

    compiler.add_return_val(&return_val, false)?;

    compiler.pop_context()?;
    Ok(())
}

fn add_get_mut(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    field: &FieldInfo,
) -> Result<()> {
    let field_name = field.get_name();
    let rust_type = fd.get_rust_type(field.clone())?;
    let func_context = FunctionContext::new(
        &format!("get_mut_{}", &field_name),
        true,
        vec![FunctionArg::MutSelfArg],
        &format!("&mut {}", rust_type),
    );
    compiler.add_context(Context::Function(func_context))?;
    compiler.add_return_val(&format!("&mut self.{}", &field_name), false)?;
    compiler.pop_context()?;
    Ok(())
}

fn add_set(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    field: &FieldInfo,
) -> Result<()> {
    let field_name = field.get_name();
    let bitmap_idx_str = field.get_bitmap_idx_str(true);
    let rust_type = fd.get_rust_type(field.clone())?;
    let func_context = FunctionContext::new(
        &format!("set_{}", field_name),
        true,
        vec![
            FunctionArg::MutSelfArg,
            FunctionArg::new_arg("field", ArgInfo::owned(&rust_type)),
        ],
        "",
    );
    compiler.add_context(Context::Function(func_context))?;
    compiler.add_func_call(
        Some("self".to_string()),
        "set_bitmap_field",
        vec![format!("{}", bitmap_idx_str)],
        false,
    )?;
    compiler.add_statement(&format!("self.{}", &field_name), "field")?;
    compiler.pop_context()?;
    Ok(())
}

fn add_list_init(compiler: &mut SerializationCompiler, field: &FieldInfo) -> Result<()> {
    let func_context = FunctionContext::new(
        &format!("init_{}", field.get_name()),
        true,
        vec![
            FunctionArg::MutSelfArg,
            FunctionArg::new_arg("num", ArgInfo::owned("usize")),
        ],
        "",
    );
    compiler.add_context(Context::Function(func_context))?;
    match &field.0.typ {
        FieldType::Int32
        | FieldType::Int64
        | FieldType::Uint32
        | FieldType::Uint64
        | FieldType::Float => {
            compiler.add_statement(
                &format!("self.{}", field.get_name()),
                &format!("List::init(num)"),
            )?;
        }
        FieldType::String
        | FieldType::Bytes
        | FieldType::RefCountedString
        | FieldType::RefCountedBytes => {
            compiler.add_statement(
                &format!("self.{}", field.get_name()),
                &format!("VariableList::init(num)"),
            )?;
        }
        FieldType::MessageOrEnum(_) => {
            compiler.add_statement(
                &format!("self.{}", field.get_name()),
                &format!("VariableList::init(num)"),
            )?;
        }
        x => {
            bail!("Field type not supported: {:?}", x);
        }
    }
    compiler.add_func_call(
        Some("self".to_string()),
        "set_bitmap_field",
        vec![format!("{}", field.get_bitmap_idx_str(true))],
        false,
    )?;
    compiler.pop_context()?;
    Ok(())
}

fn add_header_repr(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
) -> Result<()> {
    let type_annotations = msg_info.get_type_params(false, &fd)?;
    let where_clause = msg_info.get_where_clause(false, &fd)?;
    let struct_name = StructName::new(&msg_info.get_name(), type_annotations.clone());
    let trait_name = TraitName::new("SgaHeaderRepr", type_annotations.clone());
    let impl_context = ImplContext::new(struct_name, Some(trait_name), where_clause);
    compiler.add_context(Context::Impl(impl_context))?;

    // add number of fields constant
    compiler.add_const_def(
        "NUMBER_OF_FIELDS",
        "usize",
        &format!("{}", msg_info.num_fields()),
    )?;
    compiler.add_newline()?;

    // add constant header size
    compiler.add_const_def("CONSTANT_HEADER_SIZE", "usize", "SIZE_FIELD + OFFSET_FIELD")?;
    compiler.add_newline()?;

    // add dynamic header size function
    let header_size_function_context = FunctionContext::new(
        "dynamic_header_size",
        false,
        vec![FunctionArg::SelfArg],
        "usize",
    );
    compiler.add_context(Context::Function(header_size_function_context))?;
    let mut dynamic_size = "BITMAP_LENGTH_FIELD + Self::bitmap_length()".to_string();
    for field in msg_info.get_fields().iter() {
        let field_info = FieldInfo(field.clone());
        let mut start = dynamic_size.to_string();
        if start.len() != 0 {
            start = format!("{} + ", start);
        }
        dynamic_size = format!(
            "{} self.get_bitmap_field({}) as usize * {}",
            start,
            &field_info.get_bitmap_idx_str(true),
            &field_info.get_total_header_size_str(true, false, true, true)?
        );
    }
    compiler.add_return_val(&dynamic_size, false)?;
    compiler.pop_context()?;
    compiler.add_newline()?;

    // add dynamic header offset function
    let header_offset_function_context = FunctionContext::new(
        "dynamic_header_start",
        false,
        vec![FunctionArg::SelfArg],
        "usize",
    );
    compiler.add_context(Context::Function(header_offset_function_context))?;
    let mut dynamic_offset = "BITMAP_LENGTH_FIELD + Self::bitmap_length()".to_string();
    for field in msg_info.get_fields().iter() {
        let field_info = FieldInfo(field.clone());
        let mut start = dynamic_offset.to_string();
        if start.len() != 0 {
            start = format!("{} + ", start);
        }
        dynamic_offset = format!(
            "{} self.get_bitmap_field({}) as usize * {}",
            start,
            &field_info.get_bitmap_idx_str(true),
            &field_info.get_header_size_str(true, false)?
        );
    }
    compiler.add_return_val(&dynamic_offset, false)?;
    compiler.pop_context()?;
    compiler.add_newline()?;

    // add num scatter_gather_entries function
    let num_sge_function_context = FunctionContext::new(
        "num_scatter_gather_entries",
        false,
        vec![FunctionArg::SelfArg],
        "usize",
    );
    compiler.add_context(Context::Function(num_sge_function_context))?;
    let mut num_sge_entries = "".to_string();
    for field in msg_info.get_fields().iter() {
        let field_info = FieldInfo(field.clone());
        let mut start = num_sge_entries.to_string();
        if start.len() != 0 {
            start = format!("{} + ", start);
        }
        match &field_info.0.typ {
            FieldType::String | FieldType::Bytes | FieldType::MessageOrEnum(_) => {
                num_sge_entries = format!(
                    "{} self.{}.num_scatter_gather_entries()",
                    start,
                    &field_info.get_name(),
                );
            }
            _ => {}
        }
    }
    compiler.add_return_val(&num_sge_entries, false)?;
    compiler.pop_context()?;
    compiler.add_newline()?;

    // add get and set bitmap fields
    let get_bitmap_context =
        FunctionContext::new("get_bitmap", false, vec![FunctionArg::SelfArg], "&[u8]");
    compiler.add_context(Context::Function(get_bitmap_context))?;
    compiler.add_return_val("&self.bitmap", false)?;
    compiler.pop_context()?;
    compiler.add_newline()?;

    let get_mut_bitmap_context = FunctionContext::new(
        "get_mut_bitmap",
        false,
        vec![FunctionArg::MutSelfArg],
        "&mut [u8]",
    );
    compiler.add_context(Context::Function(get_mut_bitmap_context))?;
    compiler.add_return_val("&mut self.bitmap", false)?;
    compiler.pop_context()?;
    compiler.add_newline()?;

    let set_bitmap_context = FunctionContext::new(
        "set_bitmap",
        false,
        vec![
            FunctionArg::MutSelfArg,
            FunctionArg::new_arg("bitmap", ArgInfo::ref_arg("[u8]", None)),
        ],
        "",
    );

    compiler.add_context(Context::Function(set_bitmap_context))?;
    compiler.add_statement("self.bitmap", "bitmap.to_vec()")?;
    compiler.pop_context()?;
    compiler.add_newline()?;

    add_equality_func(fd, compiler, msg_info)?;

    add_serialization_func(fd, compiler, msg_info)?;
    compiler.add_newline()?;

    add_deserialization_func(fd, compiler, msg_info)?;
    compiler.add_newline()?;

    compiler.pop_context()?;
    Ok(())
}

fn add_equality_func(
    _fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
) -> Result<()> {
    let func_context = FunctionContext::new(
        "check_deep_equality",
        false,
        vec![
            FunctionArg::SelfArg,
            FunctionArg::new_arg("other", ArgInfo::ref_arg("Self", None)),
        ],
        "bool",
    );
    compiler.add_context(Context::Function(func_context))?;
    for field_idx in 0..msg_info.num_fields() {
        let field_info = msg_info.get_field_from_id(field_idx as i32)?;
        let loop_context = LoopContext::new(vec![
            LoopBranch::ifbranch(&format!(
                "self.get_bitmap_field({}) != other.get_bitmap_field({})",
                field_info.get_bitmap_idx_str(true),
                field_info.get_bitmap_idx_str(true)
            )),
            LoopBranch::elseif(&format!(
                "self.get_bitmap_field({}) && other.get_bitmap_field({})",
                field_info.get_bitmap_idx_str(true),
                field_info.get_bitmap_idx_str(true)
            )),
        ]);
        compiler.add_context(Context::Loop(loop_context))?;
        compiler.add_return_val("false", true)?;
        compiler.pop_context()?;

        let check_equality_loop = {
            if field_info.is_list() {
                match &field_info.0.typ {
                    FieldType::Int32
                    | FieldType::Int64
                    | FieldType::Uint32
                    | FieldType::Uint64
                    | FieldType::Float
                    | FieldType::String
                    | FieldType::Bytes
                    | FieldType::MessageOrEnum(_) => {
                        LoopContext::new(vec![LoopBranch::ifbranch(&format!(
                            "!self.get_{}().check_deep_equality(&other.get_{}())",
                            field_info.get_name(),
                            field_info.get_name()
                        ))])
                    }
                    _ => {
                        bail!("Field type not supported: {:?}", &field_info.0.typ);
                    }
                }
            } else {
                match &field_info.0.typ {
                    FieldType::Int32
                    | FieldType::Int64
                    | FieldType::Uint32
                    | FieldType::Uint64
                    | FieldType::Float => LoopContext::new(vec![LoopBranch::ifbranch(&format!(
                        "!self.get_{}().check_deep_equality(other.get_{}())",
                        field_info.get_name(),
                        field_info.get_name()
                    ))]),
                    FieldType::String | FieldType::Bytes | FieldType::MessageOrEnum(_) => {
                        LoopContext::new(vec![LoopBranch::ifbranch(&format!(
                            "!self.get_{}().check_deep_equality(&other.get_{}())",
                            field_info.get_name(),
                            field_info.get_name()
                        ))])
                    }
                    _ => {
                        bail!("Field type not supported: {:?}", &field_info.0.typ);
                    }
                }
            }
        };

        compiler.add_context(Context::Loop(check_equality_loop))?;
        compiler.add_return_val("false", true)?;
        compiler.pop_context()?;
        compiler.pop_context()?; // outer loop
        compiler.add_newline()?;
    }

    compiler.add_return_val("true", true)?;

    compiler.pop_context()?;

    Ok(())
}

fn add_serialization_func(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
) -> Result<()> {
    let func_context = FunctionContext::new_with_lifetime(
        "inner_serialize",
        false,
        vec![
            FunctionArg::SelfArg,
            FunctionArg::new_arg("header", ArgInfo::ref_mut_arg("[u8]", None)),
            FunctionArg::new_arg("constant_header_offset", ArgInfo::owned("usize")),
            FunctionArg::new_arg("dynamic_header_start", ArgInfo::owned("usize")),
            FunctionArg::new_arg(
                "scatter_gather_entries",
                ArgInfo::ref_mut_arg("[Sge<'sge>]", None),
            ),
            FunctionArg::new_arg("offsets", ArgInfo::ref_mut_arg("[usize]", None)),
        ],
        "Result<()>",
        "'sge",
        &format!("{}: 'sge", fd.get_lifetime()),
    );

    compiler.add_context(Context::Function(func_context))?;

    // copy bitmap
    compiler.add_func_call(
        Some("self".to_string()),
        "serialize_bitmap",
        vec!["header".to_string(), "constant_header_offset".to_string()],
        false,
    )?;

    let constant_off_mut = msg_info.constant_fields_left(-1) > 1;
    compiler.add_def_with_let(
        constant_off_mut,
        None,
        "cur_constant_offset",
        "constant_header_offset + BITMAP_LENGTH_FIELD + Self::bitmap_length()",
    )?;

    compiler.add_newline()?;

    let dynamic_fields_off_mut = msg_info.dynamic_fields_left(-1, true, fd.get_message_map())? > 1;
    compiler.add_def_with_let(
        dynamic_fields_off_mut,
        None,
        "cur_dynamic_offset",
        "dynamic_header_start",
    )?;

    let string_or_bytes_fields_left = msg_info.num_string_or_bytes_fields_left(-1)?;
    let dynamic_fields_left = msg_info.dynamic_fields_left(-1, true, fd.get_message_map())?;
    let cur_sge_idx_mut = (string_or_bytes_fields_left + dynamic_fields_left) > 1;
    if msg_info.has_dynamic_fields(true, fd.get_message_map())?
        || msg_info.string_or_bytes_fields_left(-1)?
    {
        compiler.add_def_with_let(cur_sge_idx_mut, None, "cur_sge_idx", "0")?;
    }

    for field_idx in 0..msg_info.num_fields() {
        let field_info = msg_info.get_field_from_id(field_idx as i32)?;
        compiler.add_newline()?;
        let loop_context = LoopContext::new(vec![LoopBranch::ifbranch(&format!(
            "self.get_bitmap_field({})",
            field_info.get_bitmap_idx_str(true),
        ))]);
        compiler.add_context(Context::Loop(loop_context))?;
        add_serialization_for_field(fd, compiler, msg_info, &field_info)?;
        compiler.pop_context()?;
        compiler.add_newline()?;
    }

    compiler.add_return_val("Ok(())", false)?;
    compiler.pop_context()?; // function context for serialize
    Ok(())
}

fn add_deserialization_func(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
) -> Result<()> {
    let func_context = FunctionContext::new_with_lifetime(
        "inner_deserialize",
        false,
        vec![
            FunctionArg::MutSelfArg,
            FunctionArg::new_arg(
                "buffer",
                ArgInfo::ref_arg("[u8]", Some(format!("{}", fd.get_lifetime()))),
            ),
            FunctionArg::new_arg("header_offset", ArgInfo::owned("usize")),
        ],
        "Result<()>",
        "'buf",
        &format!("'buf: {}", fd.get_lifetime()),
    );
    compiler.add_context(Context::Function(func_context))?;

    // copy bitmap
    compiler.add_func_call(
        Some("self".to_string()),
        "deserialize_bitmap",
        vec!["buffer".to_string(), "header_offset".to_string()],
        false,
    )?;

    let constant_off_mut = msg_info.constant_fields_left(-1) > 1;
    compiler.add_def_with_let(
        constant_off_mut,
        None,
        "cur_constant_offset",
        "header_offset + BITMAP_LENGTH_FIELD + Self::bitmap_length()",
    )?;
    for field_idx in 0..msg_info.num_fields() {
        let field_info = msg_info.get_field_from_id(field_idx as i32)?;
        compiler.add_newline()?;
        let loop_context = LoopContext::new(vec![LoopBranch::ifbranch(&format!(
            "self.get_bitmap_field({})",
            field_info.get_bitmap_idx_str(true),
        ))]);
        compiler.add_context(Context::Loop(loop_context))?;
        add_deserialization_for_field(fd, compiler, msg_info, &field_info)?;
        compiler.pop_context()?;
        compiler.add_newline()?;
    }

    compiler.add_return_val("Ok(())", false)?;
    compiler.pop_context()?; // function context for serialize
    Ok(())
}
fn add_serialization_for_field(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
    field_info: &FieldInfo,
) -> Result<()> {
    let mut used_sge_idx = false;
    if field_info.is_list() {
        match &field_info.0.typ {
            FieldType::Int32
            | FieldType::Int64
            | FieldType::Uint32
            | FieldType::Uint64
            | FieldType::Float
            | FieldType::String
            | FieldType::Bytes
            | FieldType::MessageOrEnum(_) => {
                compiler.add_func_call(Some(format!("self.{}", field_info.get_name())), "inner_serialize", vec!["header".to_string(), "cur_constant_offset".to_string(), "cur_dynamic_offset".to_string(),
                    format!("&mut scatter_gather_entries[cur_sge_idx..(cur_sge_idx + self.{}.num_scatter_gather_entries())]", field_info.get_name()),
                    format!("&mut offsets[cur_sge_idx..(cur_sge_idx + self.{}.num_scatter_gather_entries())]", field_info.get_name())], true)?;
                used_sge_idx = true;
            }
            _ => {
                bail!("Field type not supported: {:?}", &field_info.0.typ);
            }
        }
    } else {
        match &field_info.0.typ {
            FieldType::Int32
            | FieldType::Int64
            | FieldType::Uint32
            | FieldType::Uint64
            | FieldType::Float => {
                let rust_type = &field_info.get_base_type_str()?;
                let field_size = &field_info.get_header_size_str(true, false)?;
                compiler.add_line(&format!("LittleEndian::write_{}(&header[cur_constant_offset..(cur_constant_offset + {})], {})", rust_type, field_size, &field_info.get_name()))?;
            }
            FieldType::String | FieldType::Bytes => {
                compiler.add_func_call(Some(format!("self.{}", &field_info.get_name())), "inner_serialize_with_ref", vec!["header".to_string(), "cur_constant_offset".to_string(), "cur_dynamic_offset".to_string(), format!("&mut scatter_gather_entries[cur_sge_idx..(cur_sge_idx + self.{}.num_scatter_gather_entries())]", field_info.get_name()),
                format!("&mut offsets[cur_sge_idx..(cur_sge_idx + self.{}.num_scatter_gather_entries())]", field_info.get_name()), "false".to_string()], true)?;
                used_sge_idx = true;
            }
            FieldType::MessageOrEnum(_) => {
                compiler.add_func_call(Some(format!("self.{}", &field_info.get_name())), "inner_serialize_with_ref", vec!["header".to_string(), "cur_constant_offset".to_string(), "cur_dynamic_offset".to_string(), format!("&mut scatter_gather_entries[cur_sge_idx..(cur_sge_idx + self.{}.num_scatter_gather_entries())]", field_info.get_name()),
                format!("&mut offsets[cur_sge_idx..(cur_sge_idx + self.{}.num_scatter_gather_entries())]", field_info.get_name()), "true".to_string()], true)?;
                used_sge_idx = true;
            }
            _ => {
                bail!("Field type not supported: {:?}", &field_info.0.typ);
            }
        }
    }
    compiler.add_newline()?;
    if used_sge_idx
        && ((msg_info.dynamic_fields_left(field_info.get_idx(), true, fd.get_message_map())? > 0)
            || (msg_info.string_or_bytes_fields_left(field_info.get_idx())?))
    {
        compiler.add_plus_equals(
            "cur_sge_idx",
            &format!(
                "self.{}.num_scatter_gather_entries()",
                field_info.get_name()
            ),
        )?;
    }
    if msg_info.constant_fields_left(field_info.get_idx()) > 0 {
        let field_size = &field_info.get_header_size_str(true, false)?;
        compiler.add_plus_equals("cur_constant_offset", field_size)?;
    }

    if msg_info.dynamic_fields_left(field_info.get_idx(), true, fd.get_message_map())? > 0 {
        if field_info.is_dynamic(true, fd.get_message_map())? {
            // modify cur_dynamic_ptr and cur_dynamic_offset
            compiler.add_plus_equals(
                "cur_dynamic_offset",
                &format!("self.{}.dynamic_header_size()", &field_info.get_name()),
            )?;
        }
    }

    Ok(())
}

fn add_deserialization_for_field(
    fd: &ProtoReprInfo,
    compiler: &mut SerializationCompiler,
    msg_info: &MessageInfo,
    field_info: &FieldInfo,
) -> Result<()> {
    if field_info.is_list() {
        match &field_info.0.typ {
            FieldType::Int32
            | FieldType::Int64
            | FieldType::Uint64
            | FieldType::Uint32
            | FieldType::Float
            | FieldType::String
            | FieldType::Bytes
            | FieldType::MessageOrEnum(_) => {
                compiler.add_func_call(
                    Some(format!("self.{}", field_info.get_name())),
                    "inner_deserialize",
                    vec!["buffer".to_string(), "cur_constant_offset".to_string()],
                    true,
                )?;
            }
            _ => {
                bail!("Field type {:?} not supported.", field_info.0.typ);
            }
        }
    } else {
        let rust_type = fd.get_rust_type(field_info.clone())?;
        match &field_info.0.typ {
            FieldType::Int32
            | FieldType::Int64
            | FieldType::Uint64
            | FieldType::Uint32
            | FieldType::Float => {
                compiler.add_statement(
                    &format!("self.{}", field_info.get_name()),
                    &format!("LittleEndian::read_{}(buffer[cur_constant_offset..(cur_constant_offset + {})])", rust_type, field_info.get_header_size_str(true, false)?))?;
            }
            FieldType::String | FieldType::Bytes => {
                compiler.add_func_call(
                    Some(format!("self.{}", field_info.get_name())),
                    "inner_deserialize_with_ref",
                    vec![
                        "buffer".to_string(),
                        "cur_constant_offset".to_string(),
                        "false".to_string(),
                    ],
                    true,
                )?;
            }
            FieldType::MessageOrEnum(_) => {
                compiler.add_func_call(
                    Some(format!("self.{}", field_info.get_name())),
                    "inner_deserialize_with_ref",
                    vec![
                        "buffer".to_string(),
                        "cur_constant_offset".to_string(),
                        "true".to_string(),
                    ],
                    true,
                )?;
            }
            _ => {
                bail!("Field type {:?} not supported.", field_info.0.typ);
            }
        }
    }
    if msg_info.constant_fields_left(field_info.get_idx()) > 0 {
        compiler.add_plus_equals(
            "cur_constant_offset",
            &field_info.get_header_size_str(true, false)?,
        )?;
    }
    Ok(())
}
