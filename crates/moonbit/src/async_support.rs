use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use heck::ToUpperCamelCase;
use wit_bindgen_core::{
    abi::{self, deallocate_lists_in_types, lift_from_memory, WasmSignature},
    dealias, uwriteln,
    Direction, Files, Source,
    wit_parser::{
        Function, LiftLowerAbi, ManglingAndAbi, Param, Type, TypeDefKind, TypeId, WasmImport,
    },
};

use crate::pkg::ToMoonBitIdent;
use crate::{ffi, indent, FunctionBindgen};

use super::InterfaceGenerator;

// NEW Async Impl
const ASYNC_ABI: &str = include_str!("./async/async_abi.mbt");
const ASYNC_CORO: &str = include_str!("./async/coroutine.mbt");
const ASYNC_EV: &str = include_str!("./async/ev.mbt");
const ASYNC_SCHEDULER: &str = include_str!("./async/scheduler.mbt");
const ASYNC_TASK: &str = include_str!("./async/task.mbt");
const ASYNC_TASK_GROUP: &str = include_str!("./async/task_group.mbt");
const ASYNC_TRAIT: &str = include_str!("./async/trait.mbt");
const ASYNC_PKG_JSON: &str = include_str!("./async/moon.pkg.json");
const ASYNC_PRIM: &str = include_str!("./async/async_primitive.mbt");

struct Segment<'a> {
    name: &'a str,
    src: &'a str,
}

const ASYNC_IMPL: [&Segment; 8] = [
    &Segment {
        name: "async_abi",
        src: ASYNC_ABI,
    },
    &Segment {
        name: "async_coro",
        src: ASYNC_CORO,
    },
    &Segment {
        name: "async_ev",
        src: ASYNC_EV,
    },
    &Segment {
        name: "async_scheduler",
        src: ASYNC_SCHEDULER,
    },
    &Segment {
        name: "async_task",
        src: ASYNC_TASK,
    },
    &Segment {
        name: "async_task_group",
        src: ASYNC_TASK_GROUP,
    },
    &Segment {
        name: "async_trait",
        src: ASYNC_TRAIT,
    },
    &Segment {
        name: "async_primitive",
        src: ASYNC_PRIM,
    },
];

pub(crate) const ASYNC_DIR: &str = "async";

#[derive(Default)]
pub(crate) struct AsyncSupport {
    is_async: bool,
    futures: HashMap<String, HashSet<TypeId>>,
}

impl AsyncSupport {
    pub(crate) fn mark_async(&mut self) {
        self.is_async = true;
    }

    pub(crate) fn emit_utils(&self, files: &mut Files) {
        if !self.is_async && self.futures.is_empty() {
            return;
        }

        ASYNC_IMPL.iter().for_each(|s| {
            files.push(
                &format!("{ASYNC_DIR}/{}.mbt", s.name),
                indent(s.src).as_bytes(),
            );
        });
        files.push(
            &format!("{ASYNC_DIR}/moon.pkg.json"),
            indent(ASYNC_PKG_JSON).as_bytes(),
        );
    }
}

/// lift func name, lift, lower func name, lower
pub(crate) struct AsyncBinding(pub HashMap<TypeId, (String, String, String, String)>);

/// Async-specific helpers used by `InterfaceGenerator` to keep the main
/// visitor implementation focused on shared lowering/lifting logic.
impl<'a> InterfaceGenerator<'a> {
    pub(crate) fn generate_async_import(
        &mut self,
        func: &Function,
        ffi_import_name: &str,
        wasm_sig: &WasmSignature,
    ) -> String {
        let async_pkg = self
            .world_gen
            .pkg_resolver
            .qualify_package(self.name, ASYNC_DIR);
        let param_names = func
            .params
            .iter()
            .map(|Param { name, .. }| name.to_moonbit_ident())
            .collect::<Vec<_>>();
        let param_types = func
            .params
            .iter()
            .map(|Param { ty, .. }| *ty)
            .collect::<Vec<_>>();
        let mut bindgen = FunctionBindgen::new(
            self,
            param_names.into_boxed_slice(),
            Direction::Import,
            true,
        );
        let mut lowered_params = Vec::new();

        let params_ptr = if wasm_sig.indirect_params {
            let params_info = bindgen
                .interface_gen
                .world_gen
                .sizes
                .record(param_types.iter());
            let params_ptr = bindgen.locals.tmp("params_ptr");
            bindgen.use_ffi(ffi::MALLOC);
            uwriteln!(
                bindgen.src,
                "let {params_ptr} = mbt_ffi_malloc({});",
                params_info.size.size_wasm32()
            );
            let offsets = bindgen
                .interface_gen
                .world_gen
                .sizes
                .field_offsets(param_types.iter());
            for (i, (offset, ty)) in offsets.into_iter().enumerate() {
                let param_ptr = bindgen.locals.tmp("param_ptr");
                let arg = bindgen.params[i].clone();
                uwriteln!(
                    bindgen.src,
                    "let {param_ptr} = {params_ptr} + {};",
                    offset.size_wasm32()
                );
                abi::lower_to_memory(
                    bindgen.interface_gen.resolve,
                    &mut bindgen,
                    param_ptr,
                    arg,
                    ty,
                );
            }
            lowered_params.push(params_ptr.clone());
            Some(params_ptr)
        } else {
            for (i, ty) in param_types.iter().enumerate() {
                let arg = bindgen.params[i].clone();
                lowered_params.extend(abi::lower_flat(
                    bindgen.interface_gen.resolve,
                    &mut bindgen,
                    arg,
                    ty,
                ));
            }
            None
        };
        let cleaned = bindgen.locals.tmp("cleaned");
        uwriteln!(bindgen.src, "let {cleaned} : Ref[Bool] = {{ val: false }}");

        let results_ptr = if func.result.is_some() {
            let result_info = bindgen.interface_gen.world_gen.sizes.params(&func.result);
            let results_ptr = bindgen.locals.tmp("results_ptr");
            bindgen.use_ffi(ffi::MALLOC);
            bindgen.use_ffi(ffi::FREE);
            uwriteln!(
                bindgen.src,
                "let {results_ptr} = mbt_ffi_malloc({});\n\
defer mbt_ffi_free({results_ptr})",
                result_info.size.size_wasm32()
            );
            Some(results_ptr)
        } else {
            None
        };

        let mut call_args = lowered_params.clone();
        if let Some(results_ptr) = &results_ptr {
            call_args.push(results_ptr.clone());
        }
        let subtask = bindgen.locals.tmp("subtask");
        uwriteln!(
            bindgen.src,
            "let {subtask} = {ffi_import_name}({});",
            call_args.join(", ")
        );

        let cleanup_params = bindgen.locals.tmp("cleanup_params");
        uwriteln!(
            bindgen.src,
            "fn {cleanup_params}() -> Unit {{\n  if {cleaned}.val {{ return }}\n  {cleaned}.val = true"
        );
        let dealloc_operands = if wasm_sig.indirect_params {
            vec![params_ptr.clone().unwrap()]
        } else {
            lowered_params.clone()
        };
        deallocate_lists_in_types(
            bindgen.interface_gen.resolve,
            &param_types,
            &dealloc_operands,
            wasm_sig.indirect_params,
            &mut bindgen,
        );
        if let Some(params_ptr) = &params_ptr {
            bindgen.use_ffi(ffi::FREE);
            uwriteln!(bindgen.src, "  mbt_ffi_free({params_ptr})");
        }
        uwriteln!(
            bindgen.src,
            "}}\nfn cleanup_after_started() -> Unit {{ {cleanup_params}() }}\n\
defer {cleanup_params}()\n{async_pkg}suspend_for_subtask({subtask}, cleanup_after_started)",
        );

        if let Some(result) = func.result {
            let lifted = lift_from_memory(
                bindgen.interface_gen.resolve,
                &mut bindgen,
                results_ptr.clone().unwrap(),
                &result,
            );
            uwriteln!(bindgen.src, "return {lifted}");
        }

        bindgen.src
    }

    /// Generate the async bindings for this function.
    ///
    /// Note that these bindings may be referenced while generating other async
    /// bindings (e.g. `future<record { field: future<T> }>`), so this method
    /// populates `self.bindings` incrementally.
    pub(crate) fn generate_async_binding(&mut self, func: &Function) {
        self.bindings.0.clear();
        let futures_and_streams = func.find_futures_and_streams(self.resolve);
        let (module, func_name) = self.resolve.wasm_import_name(
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
            WasmImport::Func {
                interface: self.interface,
                func,
            },
        );
        for (idx, type_) in futures_and_streams.iter().enumerate() {
            let ty = dealias(self.resolve, *type_);
            match self.resolve.types[ty].kind {
                TypeDefKind::Future(_) => {
                    let binding = self.generate_future_binding(ty, idx, &module, &func_name);
                    self.bindings.0.insert(ty, binding);
                }
                TypeDefKind::Stream(_) => {
                    let binding = self.generate_stream_binding(ty, idx, &module, &func_name);
                    self.bindings.0.insert(ty, binding);
                }
                _ => unreachable!("Expected future and stream"),
            }
        }
    }

    pub(crate) fn generate_future_binding(
        &mut self,
        ty: TypeId,
        index: usize,
        module: &str,
        func_name: &str,
    ) -> (String, String, String, String) {
        let mut lift = Source::default();
        let mut lower = Source::default();

        let camel_name = func_name.to_upper_camel_case();
        let lifted_func_name = format!("wasmLift{camel_name}{index}");
        let lowered_func_name = format!("wasmLower{camel_name}{index}");
        let async_qualifier = self
            .world_gen
            .pkg_resolver
            .qualify_package(self.name, ASYNC_DIR);
        let module = if self.direction == Direction::Export && !module.starts_with("[export]") {
            format!("[export]{module}")
        } else {
            module.to_string()
        };
        let lifted = self
            .world_gen
            .pkg_resolver
            .type_name(self.name, &Type::Id(ty));

        // write intrinsics
        uwriteln!(
            lift,
            r#"
fn wasmLift{camel_name}{index}Read(handle : Int, ptr : Int) -> Int = "{module}" "[async-lower][future-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}CancelRead(_ : Int) -> Int = "{module}" "[future-cancel-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}DropReadable(_ : Int) = "{module}" "[future-drop-readable-{index}]{func_name}"
    "#,
        );
        uwriteln!(
            lower,
            r#"
fn wasmLower{camel_name}{index}New() -> UInt64 = "{module}" "[future-new-{index}]{func_name}"
fn wasmLower{camel_name}{index}Write(handle : Int, ptr : Int) -> Int = "{module}" "[future-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}CancelWrite(_ : Int) -> Int = "{module}" "[future-cancel-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}DropWritable(_ : Int) = "{module}" "[future-drop-writable-{index}]{func_name}"
    "#
        );

        // generate function
        let size = if let TypeDefKind::Future(Some(inner_ty)) = self.resolve.types[ty].kind {
            self.world_gen.sizes.size(&inner_ty).size_wasm32()
        } else {
            0
        };
        uwriteln!(
            lift,
            r#"
fn wasmLift{camel_name}{index}(future_handle : Int) -> {lifted} {{
  let mut result = None
  let mut dropped = false
  let mut reading = 0
  async fn drop() {{
    if !dropped && reading > 0 {{
      {async_qualifier}suspend_for_future_read(
        future_handle,
        wasmLift{camel_name}{index}CancelRead(future_handle)
      ) catch {{ 
       {async_qualifier}FutureReadError::Cancelled => ()
       _ => panic() 
      }}
    }}
    if !dropped {{
      dropped = true
      wasmLift{camel_name}{index}DropReadable(future_handle)
    }}
  }}
  {async_qualifier}Future::Incoming({async_qualifier}FutureR::{{
    handle: future_handle,
    get: fn () {{
      if result is Some(r) {{
        return r
      }}
      if dropped {{
        raise {async_qualifier}FutureReadError::Dropped
      }}
      let ptr = mbt_ffi_malloc({size})
      defer mbt_ffi_free(ptr)
      {{
        reading += 1
        defer {{ reading -= 1 }}
        {async_qualifier}suspend_for_future_read(
          future_handle,
          wasmLift{camel_name}{index}Read(future_handle, ptr),
        )
      }}
      result = {{
      "#
        );
        let operand = if let TypeDefKind::Future(Some(ty)) = self.resolve.types[ty].kind {
            // TODO : solve ownership
            let resolve = self.resolve.clone();
            let mut bindgen = FunctionBindgen::new(self, Box::new([]), Direction::Import, true);
            let operand = lift_from_memory(&resolve, &mut bindgen, "ptr".to_string(), &ty);
            uwriteln!(lift, "{}", bindgen.src);
            operand
        } else {
            "()".into()
        };

        // lift from memory if it were actual data
        uwriteln!(
            lift,
            r#"
        Some({operand})
      }}
      drop()
      result.unwrap()
    }},
    drop,
    take_handle: fn () {{
      if dropped || reading > 0 {{
        panic()
      }}
      dropped = true
      future_handle
    }}
  }})
}}
"#
        );

        // Generate the lower function body
        let inner_type = if let TypeDefKind::Future(Some(inner_ty)) = self.resolve.types[ty].kind {
            Some(inner_ty)
        } else {
            None
        };

        uwriteln!(
            lower,
            r#"
fn wasmLower{camel_name}{index}(future : {lifted}) -> Int {{
  match future {{
    {async_qualifier}Future::Incoming(f) => f.take_handle()
    {async_qualifier}Future::Outgoing(f) => {{
      let handles = wasmLower{camel_name}{index}New()
      let readable = (handles & 0xFFFFFFFF).to_int()
      let writable = (handles >> 32).to_int()
      let producer = f.take_producer()
      {async_qualifier}backpressure_inc()
      {async_qualifier}spawn_bg_current(async fn() {{
        defer {async_qualifier}backpressure_dec()
        defer wasmLower{camel_name}{index}DropWritable(writable)"#
        );

        if let Some(inner_ty) = inner_type {
            let resolve = self.resolve.clone();
            let mut bindgen = FunctionBindgen::new(self, Box::new([]), Direction::Export, true);
            bindgen.use_ffi(ffi::MALLOC);
            bindgen.use_ffi(ffi::FREE);
            uwriteln!(
                lower,
                r#"
        let value = producer()
    let ret_area = mbt_ffi_malloc({size})
    defer mbt_ffi_free(ret_area)"#
            );
            abi::lower_to_memory(
                &resolve,
                &mut bindgen,
                "ret_area".to_string(),
                "value".to_string(),
                &inner_ty,
            );
            uwriteln!(lower, "{}", bindgen.src);
            uwriteln!(
                lower,
                r#"
        let _ = {async_qualifier}suspend_for_future_write(writable, wasmLower{camel_name}{index}Write(writable, ret_area)) catch {{ _ => false }}"#
            );
        } else {
            // Unit type - no value to write, just complete the future
            uwriteln!(
                lower,
                r#"
        let _ = producer()
        let _ = {async_qualifier}suspend_for_future_write(writable, wasmLower{camel_name}{index}Write(writable, 0)) catch {{ _ => false }}"#
            );
        }

        uwriteln!(
            lower,
            r#"
      }})
      readable
    }}
  }}
}}"#
        );
        (
            lifted_func_name,
            lift.to_string(),
            lowered_func_name,
            lower.to_string(),
        )
    }

    pub(crate) fn generate_stream_binding(
        &mut self,
        ty: TypeId,
        index: usize,
        module: &str,
        func_name: &str,
    ) -> (String, String, String, String) {
        let mut lift = Source::default();
        let mut lower = Source::default();

        let camel_name = func_name.to_upper_camel_case();
        let lifted_func_name = format!("wasmLift{camel_name}{index}");
        let lowered_func_name = format!("wasmLower{camel_name}{index}");
        let async_qualifier = self
            .world_gen
            .pkg_resolver
            .qualify_package(self.name, ASYNC_DIR);
        let module = if self.direction == Direction::Export && !module.starts_with("[export]") {
            format!("[export]{module}")
        } else {
            module.to_string()
        };
        let lifted = self
            .world_gen
            .pkg_resolver
            .type_name(self.name, &Type::Id(ty));

        // write intrinsics
        uwriteln!(
            lift,
            r#"
fn wasmLift{camel_name}{index}Read(handle : Int, ptr : Int, len : Int) -> Int = "{module}" "[async-lower][stream-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}CancelRead(_ : Int) -> Int = "{module}" "[stream-cancel-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}DropReadable(_ : Int) = "{module}" "[stream-drop-readable-{index}]{func_name}"
    "#,
        );
        uwriteln!(
            lower,
            r#"
fn wasmLower{camel_name}{index}New() -> UInt64 = "{module}" "[stream-new-{index}]{func_name}"
fn wasmLower{camel_name}{index}Write(handle : Int, ptr : Int, len : Int) -> Int = "{module}" "[stream-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}CancelWrite(_ : Int) -> Int = "{module}" "[stream-cancel-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}DropWritable(_ : Int) = "{module}" "[stream-drop-writable-{index}]{func_name}"
    "#
        );

        // Get element type and size
        let inner_type = if let TypeDefKind::Stream(Some(inner_ty)) = self.resolve.types[ty].kind {
            Some(inner_ty)
        } else {
            None
        };
        let elem_size = inner_type
            .map(|t| self.world_gen.sizes.size(&t).size_wasm32())
            .unwrap_or(0);

        // Generate lift function (StreamR from handle)
        uwriteln!(
            lift,
            r#"
fn wasmLift{camel_name}{index}(stream_handle : Int) -> {lifted} {{
  let mut closed = false
  let mut reading = 0
  async fn close() {{
    if !closed && reading > 0 {{
      let _ = {async_qualifier}suspend_for_stream_read(
        stream_handle,
        wasmLift{camel_name}{index}CancelRead(stream_handle)
      ) catch {{ _ => (0, false) }}
    }}
    if !closed {{
      closed = true
      wasmLift{camel_name}{index}DropReadable(stream_handle)
    }}
  }}
  {async_qualifier}Stream::Incoming({async_qualifier}StreamR::{{
    handle: stream_handle,
    read: fn (count : Int) {{
      if closed {{
        return None
      }}"#
        );

        if let Some(inner_ty) = inner_type {
            let resolve = self.resolve.clone();
            let mut lift_bindgen =
                FunctionBindgen::new(self, Box::new([]), Direction::Import, true);
            lift_bindgen.use_ffi(ffi::MALLOC);
            lift_bindgen.use_ffi(ffi::FREE);

            uwriteln!(
                lift,
                r#"
      let ptr = mbt_ffi_malloc(count * {elem_size})
      reading += 1
      let (progress, end) = {{
        defer {{ reading -= 1 }}
        {async_qualifier}suspend_for_stream_read(
          stream_handle,
          wasmLift{camel_name}{index}Read(stream_handle, ptr, count),
        )
      }}
      if progress == 0 {{
        mbt_ffi_free(ptr)
        if end {{ close(); return None }}
        return Some([])
      }}
      let items = []"#
            );

            // Generate code to lift each element from memory
            uwriteln!(lift, "      for i = 0; i < progress; i = i + 1 {{");
            uwriteln!(lift, "        let elem_ptr = ptr + i * {elem_size}");
            let operand = lift_from_memory(
                &resolve,
                &mut lift_bindgen,
                "elem_ptr".to_string(),
                &inner_ty,
            );
            uwriteln!(lift, "{}", lift_bindgen.src);
            uwriteln!(lift, "        items.push({operand})");
            uwriteln!(lift, "      }}");

            uwriteln!(
                lift,
                r#"
      mbt_ffi_free(ptr)
      if end {{ close() }}
      Some(items[:])"#
            );
        } else {
            // Unit type stream
            uwriteln!(
                lift,
                r#"
      reading += 1
      let (progress, end) = {{
        defer {{ reading -= 1 }}
        {async_qualifier}suspend_for_stream_read(
          stream_handle,
          wasmLift{camel_name}{index}Read(stream_handle, 0, count),
        )
      }}
      if progress == 0 && end {{ close(); return None }}
      let result = FixedArray::make(progress, ())
      if end {{ close() }}
      Some(result[:])"#
            );
        }

        uwriteln!(
            lift,
            r#"
    }},
    close,
    take_handle: fn () {{
      if closed || reading > 0 {{
        panic()
      }}
      closed = true
      stream_handle
    }}
  }})
}}"#
        );

        // Generate lower function (Stream to handle)
        uwriteln!(
            lower,
            r#"
fn wasmLower{camel_name}{index}(stream : {lifted}) -> Int {{
  match stream {{
    {async_qualifier}Stream::Incoming(s) => s.take_handle()
    {async_qualifier}Stream::Outgoing(s) => {{
      let handles = wasmLower{camel_name}{index}New()
      let readable = (handles & 0xFFFFFFFF).to_int()
      let writable = (handles >> 32).to_int()
      let producer = s.take_producer()
      {async_qualifier}backpressure_inc()
      let _ = {async_qualifier}spawn_bg_current(async fn() {{
        defer {async_qualifier}backpressure_dec()
        let mut closed = false
        defer {{
          if !closed {{
            wasmLower{camel_name}{index}DropWritable(writable)
          }}
        }}
        let sink = {async_qualifier}Sink::{{
          write: async fn (data : ArrayView[_]) {{
            if closed || data.length() == 0 {{
              return 0
            }}"#
        );

        if let Some(inner_ty) = inner_type {
            let resolve = self.resolve.clone();
            let elem_type = self.world_gen.pkg_resolver.type_name(self.name, &inner_ty);
            let mut lower_bindgen =
                FunctionBindgen::new(self, Box::new([]), Direction::Export, true);
            lower_bindgen.use_ffi(ffi::MALLOC);
            lower_bindgen.use_ffi(ffi::FREE);

            uwriteln!(
                lower,
                r#"
            let ptr = mbt_ffi_malloc(data.length() * {elem_size})
            defer mbt_ffi_free(ptr)
            for i = 0; i < data.length(); i = i + 1 {{
              let elem_ptr = ptr + i * {elem_size}
              let elem : {elem_type} = data[i]"#
            );

            abi::lower_to_memory(
                &resolve,
                &mut lower_bindgen,
                "elem_ptr".to_string(),
                "elem".to_string(),
                &inner_ty,
            );
            uwriteln!(lower, "{}", lower_bindgen.src);
            uwriteln!(lower, "            }}");

            uwriteln!(
                lower,
                r#"
            let (progress, dropped) = {async_qualifier}suspend_for_stream_write(
              writable,
              wasmLower{camel_name}{index}Write(writable, ptr, data.length()),
            ) catch {{ _ => (0, true) }}
            if dropped {{
              closed = true
              wasmLower{camel_name}{index}DropWritable(writable)
            }}
            progress"#
            );
        } else {
            // Unit type stream
            uwriteln!(
                lower,
                r#"
            let (progress, dropped) = {async_qualifier}suspend_for_stream_write(
              writable,
              wasmLower{camel_name}{index}Write(writable, 0, data.length()),
            ) catch {{ _ => (0, true) }}
            if dropped {{
              closed = true
              wasmLower{camel_name}{index}DropWritable(writable)
            }}
            progress"#
            );
        }

        uwriteln!(
            lower,
            r#"
          }},
          close: async fn () {{
            if !closed {{
              closed = true
              wasmLower{camel_name}{index}DropWritable(writable)
            }}
          }}
        }}
        producer(sink)
        sink.close()
      }})
      readable
    }}
  }}
}}"#
        );

        (
            lifted_func_name,
            lift.to_string(),
            lowered_func_name,
            lower.to_string(),
        )
    }
}
