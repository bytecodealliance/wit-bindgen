use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use heck::{ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_core::{
    Files, Source,
    abi::{self, WasmSignature},
    uwriteln,
    wit_parser::{Function, Resolve, Type, TypeDefKind, TypeId},
};

use crate::{
    FFI, FFI_DIR, indent,
    pkg::{MoonbitSignature, ToMoonBitIdent},
};

use super::{FunctionBindgen, InterfaceGenerator, PayloadFor};

const ASYNC_PRIMITIVE: &str = include_str!("./ffi/async_primitive.mbt");
const ASYNC_FUTURE: &str = include_str!("./ffi/future.mbt");
const ASYNC_WASM_PRIMITIVE: &str = include_str!("./ffi/wasm_primitive.mbt");
const ASYNC_WAITABLE_SET: &str = include_str!("./ffi/waitable_task.mbt");
const ASYNC_SUBTASK: &str = include_str!("./ffi/subtask.mbt");

struct Segment<'a> {
    name: &'a str,
    src: &'a str,
}

const ASYNC_UTILS: [&Segment; 5] = [
    &Segment {
        name: "async_primitive",
        src: ASYNC_PRIMITIVE,
    },
    &Segment {
        name: "async_future",
        src: ASYNC_FUTURE,
    },
    &Segment {
        name: "async_wasm_primitive",
        src: ASYNC_WASM_PRIMITIVE,
    },
    &Segment {
        name: "async_waitable_set",
        src: ASYNC_WAITABLE_SET,
    },
    &Segment {
        name: "async_subtask",
        src: ASYNC_SUBTASK,
    },
];

#[derive(Default)]
pub(crate) struct AsyncSupport {
    is_async: bool,
    futures: HashMap<String, HashSet<TypeId>>,
}

impl AsyncSupport {
    pub(crate) fn mark_async(&mut self) {
        self.is_async = true;
    }

    pub(crate) fn register_future_or_stream(&mut self, module: &str, ty: TypeId) -> bool {
        self.futures
            .entry(module.to_string())
            .or_default()
            .insert(ty)
    }

    pub(crate) fn emit_utils(&self, files: &mut Files, version: &str) {
        if !self.is_async && self.futures.is_empty() {
            return;
        }

        let mut body = Source::default();
        wit_bindgen_core::generated_preamble(&mut body, version);
        body.push_str(FFI);
        files.push(&format!("{FFI_DIR}/top.mbt"), indent(&body).as_bytes());
        ASYNC_UTILS.iter().for_each(|s| {
            files.push(
                &format!("{FFI_DIR}/{}.mbt", s.name),
                indent(s.src).as_bytes(),
            );
        });
        files.push(
            &format!("{FFI_DIR}/moon.pkg.json"),
            "{ \"warn-list\": \"-44\", \"supported-targets\": [\"wasm\"] }".as_bytes(),
        );
    }
}

/// Async-specific helpers used by `InterfaceGenerator` to keep the main
/// visitor implementation focused on shared lowering/lifting logic.
impl<'a> InterfaceGenerator<'a> {
    /// Builds the MoonBit body for async imports, wiring wasm subtasks into the
    /// runtime and lowering/lifting payloads as needed.
    pub(super) fn generate_async_import_function(
        &mut self,
        func: &Function,
        mbt_sig: MoonbitSignature,
        sig: &WasmSignature,
    ) -> String {
        let mut body = String::default();
        let mut lower_params = Vec::new();
        let mut lower_results = Vec::new();

        if sig.indirect_params {
            match &func.params[..] {
                [] => {}
                [(_, _)] => {
                    lower_params.push("_lower_ptr".into());
                }
                multiple_params => {
                    let params = multiple_params.iter().map(|(_, ty)| ty);
                    let offsets = self.r#gen.sizes.field_offsets(params.clone());
                    let elem_info = self.r#gen.sizes.params(params);
                    body.push_str(&format!(
                        r#"
                        let _lower_ptr : Int = {ffi}malloc({})
                        "#,
                        elem_info.size.size_wasm32(),
                        ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR)
                    ));

                    for ((offset, ty), name) in offsets.iter().zip(
                        multiple_params
                            .iter()
                            .map(|(name, _)| name.to_moonbit_ident()),
                    ) {
                        let result = self.lower_to_memory(
                            &format!("_lower_ptr + {}", offset.size_wasm32()),
                            &name,
                            ty,
                            self.name,
                        );
                        body.push_str(&result);
                    }

                    lower_params.push("_lower_ptr".into());
                }
            }
        } else {
            let mut f = FunctionBindgen::new(self, "INVALID", self.name, Box::new([]));
            for (name, ty) in mbt_sig.params.iter() {
                lower_params.extend(abi::lower_flat(f.r#gen.resolve, &mut f, name.clone(), ty));
            }
            lower_results.push(f.src.clone());
        }

        let func_name = func.name.to_upper_camel_case();

        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);

        let call_import = |params: &Vec<String>| {
            format!(
                r#"
                let _subtask_code = wasmImport{func_name}({})
                let _subtask_status = {ffi}SubtaskStatus::decode(_subtask_code)
                let _subtask = @ffi.Subtask::from_handle(_subtask_status.handle(), code=_subtask_code)

                let task = @ffi.current_task()
                task.add_waitable(_subtask, @ffi.current_coroutine())
                defer task.remove_waitable(_subtask)

                for {{
                        if _subtask.done() || _subtask_status is Returned(_) {{
                            break
                        }} else {{
                            @ffi.suspend()
                        }}
                    }}

                "#,
                params.join(", ")
            )
        };
        match &func.result {
            Some(ty) => {
                lower_params.push("_result_ptr".into());
                let call_import = call_import(&lower_params);
                let (lift, lift_result) = &self.lift_from_memory("_result_ptr", ty, self.name);
                body.push_str(&format!(
                    r#"
                    {}
                    {}
                    {call_import}
                    {lift}
                    {lift_result}
                    "#,
                    lower_results.join("\n"),
                    &self.malloc_memory("_result_ptr", "1", ty)
                ));
            }
            None => {
                let call_import = call_import(&lower_params);
                body.push_str(&call_import);
            }
        }

        body.to_string()
    }

    /// Ensures async futures and streams referenced by `func` have their helper
    /// import tables generated for the given module prefix.
    pub(super) fn generation_futures_and_streams_import(
        &mut self,
        prefix: &str,
        func: &Function,
        module: &str,
    ) {
        let module = format!("{prefix}{module}");
        for (index, ty) in func
            .find_futures_and_streams(self.resolve)
            .into_iter()
            .enumerate()
        {
            let func_name = &func.name;

            match &self.resolve.types[ty].kind {
                TypeDefKind::Future(payload_type) => {
                    self.r#generate_async_future_or_stream_import(
                        PayloadFor::Future,
                        &module,
                        index,
                        func_name,
                        ty,
                        payload_type.as_ref(),
                    );
                }
                TypeDefKind::Stream(payload_type) => {
                    self.r#generate_async_future_or_stream_import(
                        PayloadFor::Stream,
                        &module,
                        index,
                        func_name,
                        ty,
                        payload_type.as_ref(),
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    fn generate_async_future_or_stream_import(
        &mut self,
        payload_for: PayloadFor,
        module: &str,
        index: usize,
        func_name: &str,
        ty: TypeId,
        result_type: Option<&Type>,
    ) {
        if !self
            .r#gen
            .async_support
            .register_future_or_stream(module, ty)
        {
            return;
        }
        let result = match result_type {
            Some(ty) => self.r#gen.pkg_resolver.type_name(self.name, ty),
            None => "Unit".into(),
        };

        let type_name = self.r#gen.pkg_resolver.type_name(self.name, &Type::Id(ty));
        let name = result.to_upper_camel_case();
        let kind = match payload_for {
            PayloadFor::Future => "future",
            PayloadFor::Stream => "stream",
        };
        let table_name = format!("{}_{}_table", type_name.to_snake_case(), kind);
        let camel_kind = kind.to_upper_camel_case();
        let payload_len_arg = match payload_for {
            PayloadFor::Future => "",
            PayloadFor::Stream => " ,length : Int",
        };

        let payload_lift_func = match payload_for {
            PayloadFor::Future => "",
            PayloadFor::Stream => "List",
        };
        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);

        let mut dealloc_list;
        let malloc;
        let lift;
        let lower;
        let lift_result;
        let lift_list: String;
        let lower_list: String;
        if let Some(result_type) = result_type {
            (lift, lift_result) = self.lift_from_memory("ptr", result_type, module);
            lower = self.lower_to_memory("ptr", "value", result_type, module);
            dealloc_list = self.deallocate_lists(
                std::slice::from_ref(result_type),
                &[String::from("ptr")],
                true,
                module,
            );
            lift_list = self.list_lift_from_memory(
                "ptr",
                "length",
                &format!("wasm{name}{kind}Lift"),
                result_type,
            );
            lower_list =
                self.list_lower_to_memory(&format!("wasm{name}{kind}Lower"), "value", result_type);

            malloc = self.malloc_memory("ptr", "length", result_type);

            if dealloc_list.is_empty() {
                dealloc_list = "let _ = ptr".to_string();
            }
        } else {
            lift = "let _ = ptr".to_string();
            lower = "let _ = (ptr, value)".to_string();
            dealloc_list = "let _ = ptr".to_string();
            malloc = "let ptr = 0;".into();
            lift_result = "".into();
            lift_list = "FixedArray::make(length, Unit::default())".into();
            lower_list = "0".into();
        }

        let (mut lift_func, mut lower_func) = if result_type
            .is_some_and(|ty| self.is_list_canonical(self.resolve, ty))
            && matches!(payload_for, PayloadFor::Stream)
        {
            ("".into(), "".into())
        } else {
            (
                format!(
                    r#"
                fn wasm{name}{kind}Lift(ptr: Int) -> {result} {{
                    {lift}
                    {lift_result}
                }}
                "#
                ),
                format!(
                    r#"
                fn wasm{name}{kind}Lower(value: {result}, ptr: Int) -> Unit {{
                    {lower}
                }}
                "#
                ),
            )
        };

        if matches!(payload_for, PayloadFor::Stream) {
            lift_func.push_str(&format!(
                r#"
                fn wasm{name}{kind}ListLift(ptr: Int, length: Int) -> FixedArray[{result}] {{ 
                    {lift_list}
                }}
                "#
            ));

            lower_func.push_str(&format!(
                r#"
                fn wasm{name}{kind}ListLower(value: FixedArray[{result}]) -> Int {{ 
                    {lower_list}
                }}
                "#
            ));
        };

        uwriteln!(
            self.ffi,
            r#"
fn wasmImport{name}{kind}New() -> UInt64 = "{module}" "[{kind}-new-{index}]{func_name}"
fn wasmImport{name}{kind}Read(handle : Int, buffer_ptr : Int{payload_len_arg}) -> Int = "{module}" "[async-lower][{kind}-read-{index}]{func_name}"
fn wasmImport{name}{kind}Write(handle : Int, buffer_ptr : Int{payload_len_arg}) -> Int = "{module}" "[async-lower][{kind}-write-{index}]{func_name}"
fn wasmImport{name}{kind}CancelRead(handle : Int) -> Int = "{module}" "[{kind}-cancel-read-{index}]{func_name}"
fn wasmImport{name}{kind}CancelWrite(handle : Int) -> Int = "{module}" "[{kind}-cancel-write-{index}]{func_name}"
fn wasmImport{name}{kind}DropReadable(handle : Int) = "{module}" "[{kind}-drop-readable-{index}]{func_name}"
fn wasmImport{name}{kind}DropWritable(handle : Int) = "{module}" "[{kind}-drop-writable-{index}]{func_name}"
fn wasm{name}{kind}Deallocate(ptr: Int) -> Unit {{
    {dealloc_list}
}}
fn wasm{name}{kind}Malloc(length: Int) -> Int {{
    {malloc}
    ptr
}}

fn {table_name}() -> {ffi}{camel_kind}VTable[{result}] {{
    {ffi}{camel_kind}VTable::new(
        wasmImport{name}{kind}New,
        wasmImport{name}{kind}Read,
        wasmImport{name}{kind}Write,
        wasmImport{name}{kind}CancelRead,
        wasmImport{name}{kind}CancelWrite,
        wasmImport{name}{kind}DropReadable,
        wasmImport{name}{kind}DropWritable,
        wasm{name}{kind}Malloc,
        wasm{name}{kind}Deallocate,
        wasm{name}{kind}{payload_lift_func}Lift,
        wasm{name}{kind}{payload_lift_func}Lower,
    )
}}
{lift_func}
{lower_func}
"#
        );
    }

    fn deallocate_lists(
        &mut self,
        types: &[Type],
        operands: &[String],
        indirect: bool,
        module: &str,
    ) -> String {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));
        abi::deallocate_lists_in_types(f.r#gen.resolve, types, operands, indirect, &mut f);
        f.src
    }

    fn lift_from_memory(&mut self, address: &str, ty: &Type, module: &str) -> (String, String) {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));

        let result = abi::lift_from_memory(f.r#gen.resolve, &mut f, address.into(), ty);
        (f.src, result)
    }

    fn lower_to_memory(&mut self, address: &str, value: &str, ty: &Type, module: &str) -> String {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));
        abi::lower_to_memory(f.r#gen.resolve, &mut f, address.into(), value.into(), ty);
        f.src
    }

    fn malloc_memory(&mut self, address: &str, length: &str, ty: &Type) -> String {
        let size = self.r#gen.sizes.size(ty).size_wasm32();
        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);
        format!("let {address} = {ffi}malloc({size} * {length});")
    }

    fn is_list_canonical(&self, _resolve: &Resolve, element: &Type) -> bool {
        matches!(
            element,
            Type::U8 | Type::U32 | Type::U64 | Type::S32 | Type::S64 | Type::F32 | Type::F64
        )
    }

    fn list_lift_from_memory(
        &mut self,
        address: &str,
        length: &str,
        lift_func: &str,
        ty: &Type,
    ) -> String {
        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);
        if self.is_list_canonical(self.resolve, ty) {
            if ty == &Type::U8 {
                return format!("{ffi}ptr2bytes({address}, {length})");
            }
            let ty = match ty {
                Type::U32 => "uint",
                Type::U64 => "uint64",
                Type::S32 => "int",
                Type::S64 => "int64",
                Type::F32 => "float",
                Type::F64 => "double",
                _ => unreachable!(),
            };

            return format!("{ffi}ptr2{ty}_array({address}, {length})");
        }
        let size = self.r#gen.sizes.size(ty).size_wasm32();
        format!(
            r#"
            FixedArray::makei(
                {length},
                (index) => {{ 
                    let ptr = ({address}) + (index * {size});
                    {lift_func}(ptr)
                }}
            )
            "#
        )
    }

    fn list_lower_to_memory(&mut self, lower_func: &str, value: &str, ty: &Type) -> String {
        // Align the address, moonbit only supports wasm32 for now
        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);
        if self.is_list_canonical(self.resolve, ty) {
            if ty == &Type::U8 {
                return format!("{ffi}bytes2ptr({value})");
            }

            let ty = match ty {
                Type::U32 => "uint",
                Type::U64 => "uint64",
                Type::S32 => "int",
                Type::S64 => "int64",
                Type::F32 => "float",
                Type::F64 => "double",
                _ => unreachable!(),
            };
            return format!("{ffi}{ty}_array2ptr({value})");
        }
        let size = self.r#gen.sizes.size(ty).size_wasm32();
        format!(
            r#"
            let address = {ffi}malloc(({value}).length() * {size});
            for index = 0; index < ({value}).length(); index = index + 1 {{
                let ptr = (address) + (index * {size});
                let value = {value}[index];
                {lower_func}(value, ptr);
            }}
            address 
            "#
        )
    }
}
