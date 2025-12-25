use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use heck::{ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_core::{
    Files, Source,
    abi::{self, WasmSignature, deallocate_lists_and_own_in_types},
    dealias, uwriteln,
    wit_parser::{
        Function, LiftLowerAbi, ManglingAndAbi, Resolve, Type, TypeDefKind, TypeId, WasmImport,
    },
};

use crate::indent;

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
    /// Generate the async bindings for this function
    pub(crate) fn generate_async_binding(&mut self, func: &Function) -> AsyncBinding {
        let mut map = HashMap::new();
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
                    map.insert(
                        ty,
                        self.generate_future_binding(ty, idx, &module, &func_name),
                    );
                }
                TypeDefKind::Stream(_) => {
                    map.insert(
                        ty,
                        self.generate_stream_binding(ty, idx, &module, &func_name),
                    );
                }
                _ => unreachable!("Expected future and stream"),
            }
        }
        AsyncBinding(map)
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
        let lifted = self
            .world_gen
            .pkg_resolver
            .type_name(self.name, &Type::Id(ty));
        let lowered = lifted.replace("FutureR", "OutFuture");

        // The unit case is specially handled and serves as an example
        if let TypeDefKind::Future(Option::None) = self.resolve.types[ty].kind {
            // write intrinsics
            uwriteln!(
                lift,
                r#"
fn wasmLift{camel_name}{index}Read(handle : Int, ptr : Int) -> Int = "[export]{module}" "[async-lower][future-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}CancelRead(_ : Int) -> Int = "[export]{module}" "[future-cancel-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}DropReadable(_ : Int) = "[export]{module}" "[future-drop-readable-{index}]{func_name}"
    "#,
            );
            uwriteln!(
                lower,
                r#"
fn wasmLower{camel_name}{index}New -> UInt64 = "{module}" "[future-new-{index}]{func_name}"
fn wasmLower{camel_name}{index}Write(handle : Int, ptr : Int) -> Int = "{module}" "[future-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}CancelWrite(_ : Int) -> Int = "{module}" "[future-cancel-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}DropWritable(_ : Int) = "{module}" "[future-drop-writable-{index}]{func_name}"
    "#
            );

            // generate function
            let size = self.world_gen.sizes.size(&Type::Id(ty)).size_wasm32();
            uwriteln!(
                lift,
                r#"
fn wasmLift{camel_name}{index}(future_handle : Int) -> {lifted} {{
  let mut result = None
  let mut dropped = false
  let mut reading = 0
  async fn drop() {{
    if reading > 0 {{
      {async_qualifier}suspend_for_future_read(
        future_handle,
        wasmLift{camel_name}{index}CancelRead(future_handle)
      ) catch {{ _ => () }}
    }}
    if !dropped {{
    dropped = true
    wasmLift{camel_name}{index}DropReadable(future_handle)
    }}
  }}
  {async_qualifier}FutureR::{{
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
      "#
            );
            // lift from memory if it were actual data
            uwriteln!(
                lift,
                r#"
      result = Some(())
      drop()
      result.unwrap()
    }},
    drop
  }}
}}
"#
            );

            uwriteln!(
                lower,
                r#"
fn wasmLower{camel_name}{index}(future : {lowered}) -> Int {{
  ...
}}
            "#
            );
            return (
                lifted_func_name,
                lift.to_string(),
                lowered_func_name,
                lower.to_string(),
            );
        }

        // write intrinsics
        uwriteln!(
            lift,
            r#"
fn wasmLift{camel_name}{index}Read(handle : Int, ptr : Int) -> Int = "{module}" "[future-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}CancelRead(_ : Int) -> Int = "{module}" "[future-cancel-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}DropReadable(_ : Int) = "{module}" "[future-drop-readable-{index}]{func_name}"
    "#,
        );
        uwriteln!(
            lower,
            r#"
fn wasmLower{camel_name}{index}New -> UInt64 = "{module}" "[future-new-{index}]{func_name}"
fn wasmLower{camel_name}{index}Write(handle : Int, ptr : Int) -> Int = "{module}" "[future-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}CancelWrite(_ : Int) -> Int = "{module}" "[future-cancel-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}DropWritable(_ : Int) = "{module}" "[future-drop-writable-{index}]{func_name}"
    "#
        );

        // Generate function
        uwriteln!(
            lift,
            r#"
fn wasmLift{camel_name}{index}(future_handle : Int) -> {lifted} {{
  let mut result = None
  (() => {{
    if result is Some(r) {{
      return r
    }}
    let ptr = wasmResultUnitTypesErrorCodefutureMalloc(1)
    defer mbt_ffi_free(ptr)
    {async_qualifier}suspend_for_future_read(
      future_handle,
      wasmLift{camel_name}{index}Read(future_handle, ptr),
    )
    // lift
    let lifted = wasmResultUnitTypesErrorCodefutureLift(ptr)
    result = Some(lifted)
    wasmLift{camel_name}{index}DropReadable(future_handle)
    result.unwrap()
  }})
}}
            "#
        );
        uwriteln!(
            lower,
            r#"
fn wasmLower{camel_name}{index}(future : {lowered}) -> Int {{
  ...
}}
            "#
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

        // write intrinsics
        uwriteln!(
            lift,
            r#"
fn wasmLift{camel_name}{index}Read(handle : Int, ptr : Int, len : Int) -> Int = "{module}" "[stream-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}CancelRead(_ : Int) -> Int = "{module}" "[stream-cancel-read-{index}]{func_name}"
fn wasmLift{camel_name}{index}DropReadable(_ : Int) = "{module}" "[stream-drop-readable-{index}]{func_name}"
    "#,
        );
        uwriteln!(
            lower,
            r#"
fn wasmLower{camel_name}{index}New -> UInt64 = "{module}" "[stream-new-{index}]{func_name}"
fn wasmLower{camel_name}{index}Write(handle : Int, ptr : Int, len : Int) -> Int = "{module}" "[stream-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}CancelWrite(_ : Int) -> Int = "{module}" "[stream-cancel-write-{index}]{func_name}"
fn wasmLower{camel_name}{index}DropWritable(_ : Int) = "{module}" "[stream-drop-writable-{index}]{func_name}"
    "#
        );

        (
            lifted_func_name,
            lift.to_string(),
            lowered_func_name,
            lower.to_string(),
        )
    }
}
