use std::{collections::HashSet, fmt::Write, mem, ops::Range};

use heck::{ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_core::{
    AsyncFilterSet, Direction, Files, Ns, Source,
    abi::{self, AbiVariant, WasmSignature, WasmType},
    uwrite, uwriteln,
    wit_parser::{
        Function, FutureIntrinsic, LiftLowerAbi, Mangling, ManglingAndAbi, Param, Resolve,
        StreamIntrinsic, Type, TypeDefKind, TypeId, WasmExport, WasmExportKind, WasmImport,
        WorldKey,
    },
};

use crate::{
    ffi, indent,
    pkg::{ASYNC_CORE_DIR, MoonbitSignature, ToMoonBitIdent},
};

use super::FunctionBindgen;
use super::InterfaceGenerator;
use super::wasm_type;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PayloadFor {
    Future,
    Stream,
}

impl PayloadFor {
    fn label(self) -> &'static str {
        match self {
            PayloadFor::Future => "future",
            PayloadFor::Stream => "stream",
        }
    }

    fn type_name(self) -> &'static str {
        match self {
            PayloadFor::Future => "Future",
            PayloadFor::Stream => "Stream",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AsyncFunctionState {
    task_return: AsyncTaskReturnState,
    endpoint_sites: Vec<AsyncEndpointSite>,
    endpoint_uses: Vec<EndpointUse>,
    next_endpoint: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct EndpointUse {
    lift: bool,
    lower: bool,
    lower_committed: bool,
}

#[derive(Clone, Copy, Debug)]
enum EndpointOperation {
    Lift,
    Lower,
    LowerCommitted,
}

#[derive(Clone, Debug)]
enum AsyncTaskReturnState {
    None,
    Generating {
        prev_src: String,
        prev_needs_cleanup_list: bool,
        return_param: String,
        return_value: String,
    },
    Emitted {
        params: Vec<(WasmType, String)>,
        body: String,
        needs_cleanup_list: bool,
        return_param: String,
        return_value: String,
    },
}

impl Default for AsyncTaskReturnState {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug)]
struct AsyncEndpointSite {
    kind: PayloadFor,
    ty: TypeId,
    index: usize,
    binding_name: String,
    symbol_name: String,
    payload_sites: Range<usize>,
    intrinsics: AsyncEndpointIntrinsics,
}

#[derive(Clone, Debug)]
struct AsyncIntrinsicName {
    module: String,
    field: String,
}

#[derive(Clone, Debug)]
struct AsyncEndpointIntrinsics {
    new: AsyncIntrinsicName,
    read: AsyncIntrinsicName,
    write: AsyncIntrinsicName,
    cancel_read: AsyncIntrinsicName,
    cancel_write: AsyncIntrinsicName,
    drop_readable: AsyncIntrinsicName,
    drop_writable: AsyncIntrinsicName,
}

struct EndpointPayloadFragments {
    lift: String,
    lift_result: String,
    lower: String,
    malloc: String,
    lift_list: String,
    commit: String,
    reject: String,
    free_outer: String,
}

#[derive(Clone, Debug)]
pub(crate) struct AsyncFunctionPlan {
    endpoint_sites: Vec<AsyncEndpointSite>,
}

impl AsyncFunctionPlan {
    fn new(endpoint_sites: Vec<AsyncEndpointSite>) -> Self {
        Self { endpoint_sites }
    }

    pub(crate) fn state(&self) -> AsyncFunctionState {
        AsyncFunctionState::from_sites(self.endpoint_sites.clone())
    }

    pub(crate) fn has_endpoints(&self) -> bool {
        !self.endpoint_sites.is_empty()
    }

    fn payload_sites(&self, site: &AsyncEndpointSite) -> Vec<AsyncEndpointSite> {
        self.endpoint_sites[site.payload_sites.clone()].to_vec()
    }

    fn endpoint_uses(&self, state: &AsyncFunctionState) -> Vec<EndpointUse> {
        assert_eq!(self.endpoint_sites.len(), state.endpoint_uses.len());
        let mut uses = state.endpoint_uses.clone();
        for (site_index, site) in self.endpoint_sites.iter().enumerate().rev() {
            let owner = uses[site_index];
            for index in site.payload_sites.clone() {
                uses[index].lift |= owner.lift || owner.lower;
                uses[index].lower |= owner.lower;
            }
        }
        uses
    }
}

impl AsyncFunctionState {
    fn from_sites(endpoint_sites: Vec<AsyncEndpointSite>) -> Self {
        let endpoint_uses = vec![EndpointUse::default(); endpoint_sites.len()];
        Self {
            endpoint_sites,
            endpoint_uses,
            next_endpoint: 0,
            ..Self::default()
        }
    }

    fn next_site(
        &mut self,
        kind: PayloadFor,
        ty: TypeId,
        operation: EndpointOperation,
    ) -> AsyncEndpointSite {
        let Some(offset) = self.endpoint_sites[self.next_endpoint..]
            .iter()
            .position(|site| site.kind == kind && site.ty == ty)
        else {
            unreachable!("missing async endpoint site for {kind:?} {ty:?}")
        };
        let index = self.next_endpoint + offset;
        self.next_endpoint = index + 1;
        match operation {
            EndpointOperation::Lift => self.endpoint_uses[index].lift = true,
            EndpointOperation::Lower => self.endpoint_uses[index].lower = true,
            EndpointOperation::LowerCommitted => {
                self.endpoint_uses[index].lower = true;
                self.endpoint_uses[index].lower_committed = true;
            }
        }
        self.endpoint_sites[index].clone()
    }
}

pub(crate) struct AsyncImportPlan {
    is_async: bool,
}

impl AsyncImportPlan {
    pub(crate) fn mangling_and_abi(&self) -> ManglingAndAbi {
        if self.is_async {
            ManglingAndAbi::Legacy(LiftLowerAbi::AsyncCallback)
        } else {
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync)
        }
    }

    pub(crate) fn abi_variant(&self) -> AbiVariant {
        self.mangling_and_abi().import_variant()
    }

    pub(crate) fn signature_is_async(&self) -> bool {
        self.is_async
    }

    pub(crate) fn is_async(&self) -> bool {
        self.is_async
    }
}

pub(crate) struct AsyncImportBody {
    pub(crate) src: String,
    pub(crate) needs_cleanup_list: bool,
    pub(crate) state: AsyncFunctionState,
}

pub(crate) struct AsyncExportPlan {
    is_async: bool,
}

impl AsyncExportPlan {
    pub(crate) fn mangling_and_abi(&self) -> ManglingAndAbi {
        if self.is_async {
            ManglingAndAbi::Legacy(LiftLowerAbi::AsyncCallback)
        } else {
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync)
        }
    }

    pub(crate) fn abi_variant(&self) -> AbiVariant {
        self.mangling_and_abi().export_variant()
    }

    pub(crate) fn signature_is_async(&self) -> bool {
        self.is_async
    }

    pub(crate) fn is_async(&self) -> bool {
        self.is_async
    }
}

const ASYNC_ABI: &str = include_str!("./async/async_abi.mbt");
const ASYNC_COND_VAR: &str = include_str!("./async/cond_var.mbt");
const ASYNC_COROUTINE: &str = include_str!("./async/coroutine.mbt");
const ASYNC_EV: &str = include_str!("./async/ev.mbt");
const ASYNC_MUTEX: &str = include_str!("./async/mutex.mbt");
const ASYNC_PRIMITIVE: &str = include_str!("./async/async_primitive.mbt");
const ASYNC_PROMISE: &str = include_str!("./async/promise.mbt");
const ASYNC_SCHEDULER: &str = include_str!("./async/scheduler.mbt");
const ASYNC_SEMAPHORE: &str = include_str!("./async/semaphore.mbt");
const ASYNC_TASK: &str = include_str!("./async/task.mbt");
const ASYNC_TASK_GROUP: &str = include_str!("./async/task_group.mbt");
const ASYNC_TRAIT: &str = include_str!("./async/trait.mbt");
const ASYNC_PKG: &str = include_str!("./async/moon.pkg.json");

struct Segment<'a> {
    name: &'a str,
    src: &'a str,
}

const ASYNC_UTILS: [&Segment; 12] = [
    &Segment {
        name: "async_primitive",
        src: ASYNC_PRIMITIVE,
    },
    &Segment {
        name: "async_abi",
        src: ASYNC_ABI,
    },
    &Segment {
        name: "async_coroutine",
        src: ASYNC_COROUTINE,
    },
    &Segment {
        name: "async_ev",
        src: ASYNC_EV,
    },
    &Segment {
        name: "async_cond_var",
        src: ASYNC_COND_VAR,
    },
    &Segment {
        name: "async_semaphore",
        src: ASYNC_SEMAPHORE,
    },
    &Segment {
        name: "async_mutex",
        src: ASYNC_MUTEX,
    },
    &Segment {
        name: "async_promise",
        src: ASYNC_PROMISE,
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
];

#[derive(Default)]
pub(crate) struct AsyncSupport {
    runtime_required: bool,
    endpoints: HashSet<AsyncEndpointKey>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct AsyncEndpointKey {
    binding_name: String,
    index: usize,
    kind: PayloadFor,
}

#[derive(Clone, Debug)]
struct AsyncEndpointNames {
    binding_name: String,
}

impl AsyncEndpointNames {
    fn import(resolve: &Resolve, interface: Option<&WorldKey>, func: &Function) -> Self {
        let (module, func_name) = resolve.wasm_import_name(
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
            WasmImport::Func { interface, func },
        );
        let binding_name = format!("{module}#{func_name}");
        Self { binding_name }
    }

    fn export(resolve: &Resolve, interface: Option<&WorldKey>, func: &Function) -> Self {
        let export_name = resolve.wasm_export_name(
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
            WasmExport::Func {
                interface,
                func,
                kind: WasmExportKind::Normal,
            },
        );
        Self {
            binding_name: format!("[export]{export_name}"),
        }
    }
}

impl AsyncSupport {
    pub(crate) fn is_required(&self) -> bool {
        self.runtime_required || !self.endpoints.is_empty()
    }

    pub(crate) fn import_plan(
        &mut self,
        async_filter: &mut AsyncFilterSet,
        resolve: &Resolve,
        module: Option<&WorldKey>,
        func: &Function,
    ) -> AsyncImportPlan {
        let is_async = async_filter.is_async(resolve, module, func, true);
        if is_async {
            self.runtime_required = true;
        }
        AsyncImportPlan { is_async }
    }

    pub(crate) fn export_plan(
        &mut self,
        async_filter: &mut AsyncFilterSet,
        resolve: &Resolve,
        interface: Option<&WorldKey>,
        func: &Function,
    ) -> AsyncExportPlan {
        let is_async = async_filter.is_async(resolve, interface, func, false);
        if is_async {
            self.runtime_required = true;
        }
        AsyncExportPlan { is_async }
    }

    pub(crate) fn require_runtime(&mut self) {
        self.runtime_required = true;
    }

    fn register_future_or_stream(
        &mut self,
        binding_name: &str,
        index: usize,
        kind: PayloadFor,
    ) -> bool {
        self.endpoints.insert(AsyncEndpointKey {
            binding_name: binding_name.to_string(),
            index,
            kind,
        })
    }

    pub(crate) fn emit_runtime_files(&self, files: &mut Files, version: &str) {
        if !self.is_required() {
            return;
        }

        let mut body = Source::default();
        wit_bindgen_core::generated_preamble(&mut body, version);
        files.push(
            &format!("{ASYNC_CORE_DIR}/top.mbt"),
            indent(&body).as_bytes(),
        );
        ASYNC_UTILS.iter().for_each(|s| {
            files.push(
                &format!("{ASYNC_CORE_DIR}/{}.mbt", s.name),
                s.src.as_bytes(),
            );
        });
        files.push(
            &format!("{ASYNC_CORE_DIR}/moon.pkg.json"),
            ASYNC_PKG.as_bytes(),
        );
    }
}

fn async_endpoint_sites(
    resolve: &Resolve,
    names: &AsyncEndpointNames,
    interface: Option<&WorldKey>,
    func: &Function,
    exported: bool,
) -> Vec<AsyncEndpointSite> {
    let mut sites = func
        .find_futures_and_streams(resolve)
        .into_iter()
        .enumerate()
        .map(|(index, ty)| {
            let kind = match &resolve.types[ty].kind {
                TypeDefKind::Future(_) => PayloadFor::Future,
                TypeDefKind::Stream(_) => PayloadFor::Stream,
                _ => unreachable!(),
            };
            async_endpoint_site(resolve, names, interface, func, exported, index, kind, ty)
        })
        .collect::<Vec<_>>();

    for index in 0..sites.len() {
        let payload_type = match &resolve.types[sites[index].ty].kind {
            TypeDefKind::Future(payload) | TypeDefKind::Stream(payload) => payload.as_ref(),
            _ => unreachable!(),
        };
        let Some(payload_type) = payload_type else {
            sites[index].payload_sites = index..index;
            continue;
        };
        let mut payload_types = Vec::new();
        find_futures_and_streams_in_type(resolve, payload_type, &mut payload_types);
        let start = index
            .checked_sub(payload_types.len())
            .expect("nested async endpoint payload sites must precede their owner");
        assert_eq!(
            sites[start..index]
                .iter()
                .map(|site| site.ty)
                .collect::<Vec<_>>(),
            payload_types,
            "upstream async endpoint order changed"
        );
        sites[index].payload_sites = start..index;
    }

    sites
}

fn async_endpoint_site(
    resolve: &Resolve,
    names: &AsyncEndpointNames,
    interface: Option<&WorldKey>,
    func: &Function,
    exported: bool,
    index: usize,
    kind: PayloadFor,
    ty: TypeId,
) -> AsyncEndpointSite {
    let stem = endpoint_stem(&names.binding_name, index, kind);
    AsyncEndpointSite {
        kind,
        ty,
        index,
        binding_name: names.binding_name.clone(),
        symbol_name: stem.to_upper_camel_case(),
        payload_sites: index..index,
        intrinsics: async_endpoint_intrinsics(resolve, interface, func, exported, kind, ty),
    }
}

#[derive(Clone, Copy)]
enum EndpointIntrinsic {
    New,
    Read,
    Write,
    CancelRead,
    CancelWrite,
    DropReadable,
    DropWritable,
}

impl EndpointIntrinsic {
    fn async_lowered(self) -> bool {
        matches!(self, Self::Read | Self::Write)
    }

    fn future(self) -> FutureIntrinsic {
        match self {
            Self::New => FutureIntrinsic::New,
            Self::Read => FutureIntrinsic::Read,
            Self::Write => FutureIntrinsic::Write,
            Self::CancelRead => FutureIntrinsic::CancelRead,
            Self::CancelWrite => FutureIntrinsic::CancelWrite,
            Self::DropReadable => FutureIntrinsic::DropReadable,
            Self::DropWritable => FutureIntrinsic::DropWritable,
        }
    }

    fn stream(self) -> StreamIntrinsic {
        match self {
            Self::New => StreamIntrinsic::New,
            Self::Read => StreamIntrinsic::Read,
            Self::Write => StreamIntrinsic::Write,
            Self::CancelRead => StreamIntrinsic::CancelRead,
            Self::CancelWrite => StreamIntrinsic::CancelWrite,
            Self::DropReadable => StreamIntrinsic::DropReadable,
            Self::DropWritable => StreamIntrinsic::DropWritable,
        }
    }
}

fn async_endpoint_intrinsics(
    resolve: &Resolve,
    interface: Option<&WorldKey>,
    func: &Function,
    exported: bool,
    kind: PayloadFor,
    ty: TypeId,
) -> AsyncEndpointIntrinsics {
    let intrinsic_ty = match &resolve.types[ty].kind {
        TypeDefKind::Future(None) | TypeDefKind::Stream(None) => None,
        TypeDefKind::Future(Some(_)) | TypeDefKind::Stream(Some(_)) => Some(ty),
        _ => unreachable!(),
    };
    let name = |intrinsic: EndpointIntrinsic| {
        let import = match kind {
            PayloadFor::Future => WasmImport::FutureIntrinsic {
                interface,
                func,
                ty: intrinsic_ty,
                intrinsic: intrinsic.future(),
                exported,
                async_: intrinsic.async_lowered(),
            },
            PayloadFor::Stream => WasmImport::StreamIntrinsic {
                interface,
                func,
                ty: intrinsic_ty,
                intrinsic: intrinsic.stream(),
                exported,
                async_: intrinsic.async_lowered(),
            },
        };
        let (module, field) =
            resolve.wasm_import_name(ManglingAndAbi::Legacy(LiftLowerAbi::Sync), import);
        AsyncIntrinsicName { module, field }
    };

    AsyncEndpointIntrinsics {
        new: name(EndpointIntrinsic::New),
        read: name(EndpointIntrinsic::Read),
        write: name(EndpointIntrinsic::Write),
        cancel_read: name(EndpointIntrinsic::CancelRead),
        cancel_write: name(EndpointIntrinsic::CancelWrite),
        drop_readable: name(EndpointIntrinsic::DropReadable),
        drop_writable: name(EndpointIntrinsic::DropWritable),
    }
}

fn find_futures_and_streams_in_type(resolve: &Resolve, ty: &Type, results: &mut Vec<TypeId>) {
    let Type::Id(id) = ty else {
        return;
    };

    match &resolve.types[*id].kind {
        TypeDefKind::Resource
        | TypeDefKind::Handle(_)
        | TypeDefKind::Flags(_)
        | TypeDefKind::Enum(_) => {}
        TypeDefKind::Record(record) => {
            for field in &record.fields {
                find_futures_and_streams_in_type(resolve, &field.ty, results);
            }
        }
        TypeDefKind::Tuple(tuple) => {
            for ty in &tuple.types {
                find_futures_and_streams_in_type(resolve, ty, results);
            }
        }
        TypeDefKind::Variant(variant) => {
            for case in &variant.cases {
                if let Some(ty) = &case.ty {
                    find_futures_and_streams_in_type(resolve, ty, results);
                }
            }
        }
        TypeDefKind::Option(ty)
        | TypeDefKind::List(ty)
        | TypeDefKind::FixedLengthList(ty, ..)
        | TypeDefKind::Type(ty) => find_futures_and_streams_in_type(resolve, ty, results),
        TypeDefKind::Map(key, value) => {
            find_futures_and_streams_in_type(resolve, key, results);
            find_futures_and_streams_in_type(resolve, value, results);
        }
        TypeDefKind::Result(result) => {
            if let Some(ty) = &result.ok {
                find_futures_and_streams_in_type(resolve, ty, results);
            }
            if let Some(ty) = &result.err {
                find_futures_and_streams_in_type(resolve, ty, results);
            }
        }
        TypeDefKind::Future(payload) => {
            if let Some(ty) = payload {
                find_futures_and_streams_in_type(resolve, ty, results);
            }
            results.push(*id);
        }
        TypeDefKind::Stream(payload) => {
            if let Some(ty) = payload {
                find_futures_and_streams_in_type(resolve, ty, results);
            }
            results.push(*id);
        }
        TypeDefKind::Unknown => unreachable!(),
    }
}

fn endpoint_stem(binding_name: &str, index: usize, kind: PayloadFor) -> String {
    format!(
        "{}_{}_{}",
        moonbit_identifier_stem(binding_name),
        kind.label(),
        index,
    )
}

fn moonbit_identifier_stem(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_snake_case()
}

fn background_group_name(func: &Function) -> String {
    let mut names = Ns::default();
    for param in &func.params {
        names.tmp(&param.name.to_moonbit_ident());
    }
    names.tmp("background_group")
}

/// Async-specific helpers used by `InterfaceGenerator` to keep the main
/// visitor implementation focused on shared lowering/lifting logic.
impl<'a> InterfaceGenerator<'a> {
    pub(super) fn add_async_export_stub_parameter(
        &mut self,
        func: &Function,
        is_async: bool,
        params: &mut Vec<String>,
    ) {
        if is_async && matches!(self.direction, Direction::Export) {
            let ffi = self
                .world_gen
                .pkg_resolver
                .qualify_package(self.name, ASYNC_CORE_DIR);
            let background_group = background_group_name(func);
            params.push(format!("{background_group} : {ffi}TaskGroup[Unit]"));
        }
    }

    pub(super) fn emit_async_export_wrapper(
        &mut self,
        plan: &AsyncExportPlan,
        func: &Function,
        func_name: &str,
        params: &str,
        result_type: &str,
        cleanup_list: &str,
        body: &str,
    ) -> bool {
        if !plan.is_async() {
            return false;
        }
        let ffi = self
            .world_gen
            .pkg_resolver
            .qualify_package(self.name, ASYNC_CORE_DIR);
        let background_group = background_group_name(func);
        uwrite!(
            self.ffi,
            r#"
            #doc(hidden)
            pub fn {func_name}({params}) -> {result_type} {{
                {ffi}with_waitableset(async fn() {{
                    {ffi}with_task_group(async fn({background_group}) {{
                        {cleanup_list}
                        {body}
                    }})
                }})
            }}
            "#,
        );
        true
    }

    pub(super) fn import_async_function_plan(
        &self,
        interface: Option<&WorldKey>,
        func: &Function,
    ) -> AsyncFunctionPlan {
        let names = AsyncEndpointNames::import(self.resolve, interface, func);
        AsyncFunctionPlan::new(async_endpoint_sites(
            self.resolve,
            &names,
            interface,
            func,
            false,
        ))
    }

    pub(super) fn export_async_function_plan(
        &self,
        interface: Option<&WorldKey>,
        func: &Function,
    ) -> AsyncFunctionPlan {
        let names = AsyncEndpointNames::export(self.resolve, interface, func);
        AsyncFunctionPlan::new(async_endpoint_sites(
            self.resolve,
            &names,
            interface,
            func,
            true,
        ))
    }

    pub(super) fn generate_async_import_body(
        &mut self,
        endpoint_plan: &AsyncFunctionPlan,
        func: &Function,
        mbt_sig: &MoonbitSignature,
        sig: &WasmSignature,
    ) -> AsyncImportBody {
        self.generate_async_import_function(endpoint_plan, func, mbt_sig, sig)
    }

    pub(super) fn emit_future_stream_helpers(
        &mut self,
        plan: &AsyncFunctionPlan,
        state: &AsyncFunctionState,
    ) {
        let endpoint_uses = plan.endpoint_uses(state);
        for (site, endpoint_use) in plan.endpoint_sites.iter().zip(endpoint_uses) {
            match &self.resolve.types[site.ty].kind {
                TypeDefKind::Future(payload_type) => {
                    self.generate_async_future_or_stream_import(
                        site,
                        endpoint_use,
                        plan.payload_sites(site),
                        payload_type.as_ref(),
                    );
                }
                TypeDefKind::Stream(payload_type) => {
                    self.generate_async_future_or_stream_import(
                        site,
                        endpoint_use,
                        plan.payload_sites(site),
                        payload_type.as_ref(),
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    pub(super) fn emit_async_export_callback(
        &mut self,
        plan: &AsyncExportPlan,
        interface: Option<&WorldKey>,
        func: &Function,
        camel_name: &str,
        async_state: AsyncFunctionState,
    ) -> bool {
        if !plan.is_async() {
            return false;
        }

        let export_func_name = self
            .world_gen
            .export_ns
            .tmp(&format!("wasmExportAsync{camel_name}"));
        let AsyncTaskReturnState::Emitted {
            body: task_return_body,
            needs_cleanup_list: task_return_needs_cleanup,
            params: task_return_params,
            return_param,
            return_value,
        } = async_state.task_return
        else {
            unreachable!()
        };
        let (task_return_module, task_return_name, _) =
            func.task_return_import(self.resolve, interface, Mangling::Legacy);
        let callback_export_name = self.resolve.wasm_export_name(
            plan.mangling_and_abi(),
            WasmExport::Func {
                interface,
                func,
                kind: WasmExportKind::Callback,
            },
        );
        let task_return_param_tys = task_return_params
            .iter()
            .enumerate()
            .map(|(idx, (ty, _expr))| format!("p{}: {}", idx, wasm_type(*ty)))
            .collect::<Vec<_>>()
            .join(", ");
        let task_return_param_exprs = task_return_params
            .iter()
            .map(|(_ty, expr)| expr.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let helper_package = self.name;
        let return_ty = match &func.result {
            Some(result) => self
                .world_gen
                .pkg_resolver
                .type_name(helper_package, result)
                .to_string(),
            None => "Unit".into(),
        };
        let return_expr = match return_ty.as_str() {
            "Unit" => "".into(),
            _ => format!("{return_param}: Ref[{return_ty}?]",),
        };
        let cleanup_list = if task_return_needs_cleanup {
            self.ffi_imports.insert(ffi::FREE);
            "
            let cleanup_list : Array[Int] = []
            "
        } else {
            ""
        };
        let cleanup = if task_return_needs_cleanup {
            "
            cleanup_list.each(mbt_ffi_free)
            "
        } else {
            ""
        };
        let ffi = self
            .world_gen
            .pkg_resolver
            .qualify_package(helper_package, ASYNC_CORE_DIR);
        let task_return = format!(
            r#"
            {cleanup_list}
            {task_return_body}
            {export_func_name}TaskReturn({task_return_param_exprs})
            {cleanup}
            {ffi}task_returned()
            "#
        );
        let task_return = match return_ty.as_str() {
            "Unit" => task_return,
            _ => format!(
                r#"
                match {return_param}.val {{
                    Some({return_value}) => {{
                        {task_return}
                    }}
                    None => ()
                }}
                "#
            ),
        };
        let snake_func_name = func.name.to_moonbit_ident().to_string();

        uwriteln!(
            self.ffi,
            r#"
            fn {export_func_name}TaskReturn({task_return_param_tys}) = "{task_return_module}" "{task_return_name}"

            fn {snake_func_name}_task_return({return_expr}) -> Unit {{
                {task_return}
            }}
            "#
        );

        uwriteln!(
            self.ffi,
            r#"
            #doc(hidden)
            pub fn {export_func_name}(event_raw : Int, waitable : Int, code : Int) -> Int {{
                {ffi}cb(event_raw, waitable, code)
            }}
            "#
        );

        let gen_dir = self.world_gen.opts.gen_dir.clone();
        let package = self
            .world_gen
            .pkg_resolver
            .qualify_package(&gen_dir, self.name);
        let export = format!(
            r#"
            #doc(hidden)
            pub fn {export_func_name}(event_raw : Int, waitable : Int, code : Int) -> Int {{
                {package}{export_func_name}(event_raw, waitable, code)
            }}
            "#,
        );
        self.world_gen
            .export
            .insert(callback_export_name, (export_func_name.clone(), export));

        true
    }

    /// Builds the MoonBit body for async imports, wiring wasm subtasks into the
    /// runtime and lowering/lifting payloads as needed.
    fn generate_async_import_function(
        &mut self,
        endpoint_plan: &AsyncFunctionPlan,
        func: &Function,
        mbt_sig: &MoonbitSignature,
        sig: &WasmSignature,
    ) -> AsyncImportBody {
        let mut body = String::default();
        let mut lower_params = Vec::new();
        let mut lower_results = Vec::new();
        let mut async_state = endpoint_plan.state();
        let mut needs_cleanup_list = false;
        let mut local_names = Ns::default();
        for (name, _) in &mbt_sig.params {
            local_names.tmp(name);
        }
        self.ffi_imports.insert(ffi::FREE);
        let ffi = self
            .world_gen
            .pkg_resolver
            .qualify_package(self.name, ASYNC_CORE_DIR);

        if sig.indirect_params {
            let lower_ptr = local_names.tmp("lower_ptr");
            match &func.params[..] {
                [] => {}
                [Param { name, ty, .. }] => {
                    body.push_str(&self.malloc_memory(&lower_ptr, "1", ty));
                    body.push_str(&format!("\ndefer mbt_ffi_free({lower_ptr})\n"));
                    body.push_str(&self.lower_to_memory(
                        &lower_ptr,
                        &name.to_moonbit_ident(),
                        ty,
                        self.name,
                        &mut async_state,
                    ));
                    lower_params.push(lower_ptr);
                }
                multiple_params => {
                    let params = multiple_params.iter().map(|Param { ty, .. }| ty);
                    let offsets = self.world_gen.sizes.field_offsets(params.clone());
                    let elem_info = self.world_gen.sizes.params(params);
                    self.ffi_imports.insert(ffi::MALLOC);
                    body.push_str(&format!(
                        r#"
                        let {lower_ptr} : Int = mbt_ffi_malloc({})
                        defer mbt_ffi_free({lower_ptr})
                        "#,
                        elem_info.size.size_wasm32(),
                    ));

                    for ((offset, ty), name) in offsets.iter().zip(
                        multiple_params
                            .iter()
                            .map(|Param { name, .. }| name.to_moonbit_ident()),
                    ) {
                        let result = self.lower_to_memory(
                            &format!("{lower_ptr} + {}", offset.size_wasm32()),
                            &name,
                            ty,
                            self.name,
                            &mut async_state,
                        );
                        body.push_str(&result);
                    }

                    lower_params.push(lower_ptr);
                }
            }
        } else {
            let mut f = FunctionBindgen::new(self, Box::new([]))
                .with_async_state(mem::take(&mut async_state))
                .without_block_cleanup();
            for (name, ty) in mbt_sig.params.iter() {
                lower_params.extend(abi::lower_flat(
                    f.interface_gen.resolve,
                    &mut f,
                    name.clone(),
                    ty,
                ));
            }
            lower_results.push(f.src.clone());
            needs_cleanup_list = f.needs_cleanup_list;
            async_state = f.async_state;
        }

        if !sig.indirect_params {
            let mut stable_params = Vec::with_capacity(lower_params.len());
            let bindings = lower_params
                .iter()
                .map(|value| {
                    let name = local_names.tmp("lower_arg");
                    stable_params.push(name.clone());
                    format!("let {name} = {value}")
                })
                .collect::<Vec<_>>()
                .join("\n");
            lower_results.push(bindings);
            lower_params = stable_params;
        }

        let argument_types = func
            .params
            .iter()
            .map(|Param { ty, .. }| *ty)
            .collect::<Vec<_>>();
        let argument_operands = lower_params.clone();
        let (commit_arguments, commit_state) = self.commit_lists_and_endpoints_with_state(
            &argument_types,
            &argument_operands,
            sig.indirect_params,
            self.name,
            endpoint_plan.endpoint_sites.clone(),
        );
        let (reject_arguments, rejection_state) = self.deallocate_lists_and_own_with_state(
            &argument_types,
            &argument_operands,
            sig.indirect_params,
            self.name,
            endpoint_plan.endpoint_sites.clone(),
        );
        for (used, rejected) in async_state
            .endpoint_uses
            .iter_mut()
            .zip(rejection_state.endpoint_uses)
        {
            used.lift |= rejected.lift;
            used.lower |= rejected.lower;
        }
        for (used, committed) in async_state
            .endpoint_uses
            .iter_mut()
            .zip(commit_state.endpoint_uses)
        {
            used.lift |= committed.lift;
            used.lower |= committed.lower;
        }
        let commit_arguments = if commit_arguments.trim().is_empty() {
            "()".to_string()
        } else {
            commit_arguments
        };
        let reject_arguments = if reject_arguments.trim().is_empty() {
            "()".to_string()
        } else {
            reject_arguments
        };
        let settle_arguments = format!(
            r#"
            fn(before_started) {{
                if before_started {{
                    {reject_arguments}
                }} else {{
                    {commit_arguments}
                }}
            }}
            "#
        );
        let func_name = func.name.to_upper_camel_case();

        let subtask_code = local_names.tmp("subtask_code");
        let call_import = |params: &[String], drop_returned_result: &str| {
            format!(
                r#"
                let {subtask_code} = wasmImport{func_name}({})
                {ffi}suspend_for_subtask(
                    {subtask_code},
                    {settle_arguments},
                    fn() {{
                        {drop_returned_result}
                    }},
                )

                "#,
                params.join(", ")
            )
        };
        match &func.result {
            Some(ty) => {
                let result_ptr = local_names.tmp("result_ptr");
                lower_params.push(result_ptr.clone());
                let (drop_returned_result, drop_result_state) = self
                    .deallocate_lists_and_own_with_state(
                        std::slice::from_ref(ty),
                        std::slice::from_ref(&result_ptr),
                        true,
                        self.name,
                        endpoint_plan.endpoint_sites.clone(),
                    );
                for (used, dropped) in async_state
                    .endpoint_uses
                    .iter_mut()
                    .zip(drop_result_state.endpoint_uses)
                {
                    used.lift |= dropped.lift;
                    used.lower |= dropped.lower;
                }
                let call_import = call_import(&lower_params, &drop_returned_result);
                let (lift, lift_result) =
                    &self.lift_from_memory(&result_ptr, ty, self.name, &mut async_state);
                body.push_str(&format!(
                    r#"
                    {}
                    {}
                    defer mbt_ffi_free({result_ptr})
                    {call_import}
                    {lift}
                    {lift_result}
                    "#,
                    lower_results.join("\n"),
                    &self.malloc_memory(&result_ptr, "1", ty)
                ));
            }
            None => {
                let call_import = call_import(&lower_params, "()");
                body.push_str(&format!("{}\n{call_import}", lower_results.join("\n")));
            }
        }

        AsyncImportBody {
            src: body,
            needs_cleanup_list,
            state: async_state,
        }
    }

    fn generate_async_future_or_stream_import(
        &mut self,
        site: &AsyncEndpointSite,
        endpoint_use: EndpointUse,
        payload_sites: Vec<AsyncEndpointSite>,
        result_type: Option<&Type>,
    ) {
        if !self.world_gen.async_support.register_future_or_stream(
            &site.binding_name,
            site.index,
            site.kind,
        ) {
            return;
        }
        let helper_package = self.name.to_string();
        let result = match result_type {
            Some(ty) => self.world_gen.pkg_resolver.type_name(&helper_package, ty),
            None => "Unit".into(),
        };

        let symbol_name = &site.symbol_name;
        let payload_len_arg = match site.kind {
            PayloadFor::Future => "",
            PayloadFor::Stream => ", length : Int",
        };
        let new_module = &site.intrinsics.new.module;
        let new_field = &site.intrinsics.new.field;
        let read_module = &site.intrinsics.read.module;
        let read_field = &site.intrinsics.read.field;
        let write_module = &site.intrinsics.write.module;
        let write_field = &site.intrinsics.write.field;
        let cancel_read_module = &site.intrinsics.cancel_read.module;
        let cancel_read_field = &site.intrinsics.cancel_read.field;
        let cancel_write_module = &site.intrinsics.cancel_write.module;
        let cancel_write_field = &site.intrinsics.cancel_write.field;
        let drop_readable_module = &site.intrinsics.drop_readable.module;
        let drop_readable_field = &site.intrinsics.drop_readable.field;
        let drop_writable_module = &site.intrinsics.drop_writable.module;
        let drop_writable_field = &site.intrinsics.drop_writable.field;
        let ffi = self
            .world_gen
            .pkg_resolver
            .qualify_package(&helper_package, ASYNC_CORE_DIR);

        let elem_size = result_type
            .map(|ty| self.world_gen.sizes.size(ty).size_wasm32())
            .unwrap_or(0);
        let read_chunk_owns_buffer = result_type.is_some_and(|ty| self.is_list_canonical(ty));
        let staging_window = if payload_sites.is_empty() { 64 } else { 1 };

        let EndpointPayloadFragments {
            lift,
            lift_result,
            lower,
            malloc,
            lift_list,
            commit,
            reject,
            free_outer,
        } = if let Some(result_type) = result_type {
            let mut payload_state = AsyncFunctionState::from_sites(payload_sites.clone());
            let (lift, lift_result) =
                self.lift_from_memory("ptr", result_type, &helper_package, &mut payload_state);
            let mut payload_state = AsyncFunctionState::from_sites(payload_sites.clone());
            let lower = self.lower_to_memory(
                "ptr",
                "value",
                result_type,
                &helper_package,
                &mut payload_state,
            );
            let (commit, _) = self.commit_lists_and_endpoints_with_state(
                std::slice::from_ref(result_type),
                &[String::from("elem_ptr")],
                true,
                &helper_package,
                payload_sites.clone(),
            );
            let reject = self.deallocate_lists_and_own(
                std::slice::from_ref(result_type),
                &[String::from("elem_ptr")],
                true,
                &helper_package,
                payload_sites.clone(),
            );
            let lift_list = self.list_lift_from_memory(
                "ptr",
                "length",
                &format!("wasm{symbol_name}Lift"),
                result_type,
            );
            let malloc = self.malloc_memory("ptr", "length", result_type);
            self.ffi_imports.insert(ffi::FREE);
            EndpointPayloadFragments {
                lift,
                lift_result,
                lower,
                malloc,
                lift_list,
                commit,
                reject,
                free_outer: "mbt_ffi_free(ptr)".to_string(),
            }
        } else {
            EndpointPayloadFragments {
                lift: "ignore(ptr)".into(),
                lift_result: String::new(),
                lower: "ignore((ptr, value))".into(),
                malloc: "let ptr = 0".into(),
                lift_list: "FixedArray::make(length, Unit::default())".into(),
                commit: String::new(),
                reject: String::new(),
                free_outer: "ignore(ptr)".into(),
            }
        };

        let commit = if commit.trim().is_empty() {
            "ignore((ptr, start, length))".to_string()
        } else {
            format!(
                r#"
                for i in start..<(start + length) {{
                    let elem_ptr = ptr + i * {elem_size}
                    {commit}
                }}
                "#
            )
        };
        let reject = if reject.trim().is_empty() {
            "ignore((ptr, start, length))".to_string()
        } else {
            format!(
                r#"
                for i in start..<(start + length) {{
                    let elem_ptr = ptr + i * {elem_size}
                    {reject}
                }}
                "#
            )
        };

        let lift_func = (endpoint_use.lift
            && !(matches!(site.kind, PayloadFor::Stream) && read_chunk_owns_buffer))
            .then(|| {
                format!(
                    r#"
            fn wasm{symbol_name}Lift(ptr : Int) -> {result} {{
                {lift}
                {lift_result}
            }}
            "#
                )
            })
            .unwrap_or_default();
        let lower_func = endpoint_use
            .lower
            .then(|| {
                format!(
                    r#"
            fn wasm{symbol_name}Lower(value : {result}, ptr : Int) -> Unit {{
                {lower}
            }}
            "#
                )
            })
            .unwrap_or_default();
        let list_lift_func = (endpoint_use.lift && matches!(site.kind, PayloadFor::Stream))
            .then(|| {
                format!(
                    r#"
                    fn wasm{symbol_name}ListLift(
                        ptr : Int,
                        length : Int,
                    ) -> FixedArray[{result}] {{
                        {lift_list}
                    }}
                    "#
                )
            })
            .unwrap_or_default();

        let lift_intrinsics = endpoint_use
            .lift
            .then(|| {
                format!(
                    r#"
                    fn wasmImport{symbol_name}Read(handle : Int, buffer_ptr : Int{payload_len_arg}) -> Int = "{read_module}" "{read_field}"
                    fn wasmImport{symbol_name}CancelRead(handle : Int) -> Int = "{cancel_read_module}" "{cancel_read_field}"
                    "#
                )
            })
            .unwrap_or_default();
        let drop_readable_intrinsic = (endpoint_use.lift || endpoint_use.lower)
            .then(|| {
                format!(
                    r#"
                    fn wasmImport{symbol_name}DropReadable(handle : Int) = "{drop_readable_module}" "{drop_readable_field}"
                    "#
                )
            })
            .unwrap_or_default();
        let cancel_write_intrinsic = (endpoint_use.lower
            && matches!(site.kind, PayloadFor::Stream))
        .then(|| {
            format!(
                r#"
                    fn wasmImport{symbol_name}CancelWrite(handle : Int) -> Int = "{cancel_write_module}" "{cancel_write_field}"
                    "#,
            )
        })
        .unwrap_or_default();
        let lower_intrinsics = endpoint_use
            .lower
            .then(|| {
                format!(
                    r#"
                    fn wasmImport{symbol_name}New() -> UInt64 = "{new_module}" "{new_field}"
                    fn wasmImport{symbol_name}Write(handle : Int, buffer_ptr : Int{payload_len_arg}) -> Int = "{write_module}" "{write_field}"
                    {cancel_write_intrinsic}
                    fn wasmImport{symbol_name}DropWritable(handle : Int) = "{drop_writable_module}" "{drop_writable_field}"
                    "#
                )
            })
            .unwrap_or_default();

        let commit_func = endpoint_use
            .lower
            .then(|| {
                format!(
                    r#"
                    fn wasm{symbol_name}Commit(
                        ptr : Int,
                        start : Int,
                        length : Int,
                    ) -> Unit {{
                        {commit}
                    }}
                    "#
                )
            })
            .unwrap_or_default();
        let reject_future_read = format!("wasm{symbol_name}Reject(self.read_buffer, 0, 1)");
        let reject_stream_read = format!("wasm{symbol_name}Reject(self.read_buffer, 0, progress)");

        let bridge_func = match site.kind {
            PayloadFor::Future => {
                let reject_prepared = (endpoint_use.lift && endpoint_use.lower)
                    .then(|| {
                        format!(
                            r#"
    if wasm{symbol_name}FutureRejectPrepared(self.handle) {{
        self.closed = true
        return
    }}
                            "#
                        )
                    })
                    .unwrap_or_default();
                let reject_prepared_func = (endpoint_use.lift && endpoint_use.lower)
                    .then(|| {
                        format!(
                            r#"
fn wasm{symbol_name}FutureRejectPrepared(handle : Int) -> Bool {{
    guard wasm{symbol_name}FutureProducers.get(handle) is Some(producer) else {{
        return false
    }}
    wasm{symbol_name}FutureProducers.remove(handle)
    let writer = producer.writer
    wasmImport{symbol_name}DropReadable(handle)
    {ffi}spawn_component_task_current(async fn() {{
        defer wasmImport{symbol_name}DropWritable(writer)
        {ffi}protect_from_cancel(
            () => {{
                producer.future.reject((value : {result}) => {{
                    let ptr = wasm{symbol_name}Malloc(1)
                    wasm{symbol_name}Lower(value, ptr)
                    wasm{symbol_name}Reject(ptr, 0, 1)
                    {free_outer}
                }})
                let ptr = wasm{symbol_name}Malloc(1)
                defer {{ {free_outer} }}
                let terminal : Ref[Bool?] = {{ val: None }}
                while terminal.val is None {{
                    terminal.val = {ffi}suspend_for_future_write_terminal(
                        writer,
                        wasmImport{symbol_name}Write(writer, ptr),
                    )
                }}
                guard terminal.val is Some(transferred)
                if transferred {{
                    abort("rejected component future unexpectedly transferred a value")
                }}
            }},
            resume_on_cancel=true,
        ) catch {{
            _ => abort("failed to reject component future producer")
        }}
    }})
    true
}}
                            "#
                        )
                    })
                    .unwrap_or_default();
                let lift_bridge = endpoint_use
                    .lift
                    .then(|| {
                        format!(
                            r#"
priv struct Wasm{symbol_name}FutureSource {{
    handle : Int
    mut closed : Bool
    mut reading : Bool
    mut read_task : Int
    mut read_buffer : Int
    mut read_discarding : Bool
    mut read_cleanup_done : Bool
    read_cleanup : {ffi}CondVar
}}

fn Wasm{symbol_name}FutureSource::finish_read(
    self : Wasm{symbol_name}FutureSource,
) -> Unit {{
    self.reading = false
    self.read_task = 0
    self.read_buffer = 0
    self.read_discarding = false
    // Keep completion latched until a later read starts so every waiter woken
    // by the broadcast can observe it.
}}

async fn Wasm{symbol_name}FutureSource::wait_for_read_cleanup(
    self : Wasm{symbol_name}FutureSource,
) -> Unit noraise {{
    {ffi}protect_from_cancel(
        () => {{
            while !self.read_cleanup_done {{
                self.read_cleanup.wait()
            }}
        }},
        resume_on_cancel=true,
    ) catch {{
        _ => ()
    }}
}}

async fn Wasm{symbol_name}FutureSource::cancel_active_read(
    self : Wasm{symbol_name}FutureSource,
) -> Unit noraise {{
    if !self.reading {{
        return
    }}
    if self.read_discarding {{
        self.wait_for_read_cleanup()
        return
    }}
    self.read_discarding = true
    let completed = Ref(false)
    {ffi}protect_from_cancel(
        () => {{
            completed.val = {ffi}cancel_future_read(
                self.read_task,
                self.handle,
                () => wasmImport{symbol_name}CancelRead(self.handle),
            )
        }},
        resume_on_cancel=true,
    ) catch {{
        _ => ()
    }}
    if completed.val && self.read_buffer != 0 {{
        {reject_future_read}
    }}
    self.read_cleanup_done = true
    self.read_cleanup.broadcast()
}}

async fn Wasm{symbol_name}FutureSource::close(
    self : Wasm{symbol_name}FutureSource,
) -> Unit noraise {{
    if self.closed {{
        return
    }}
    if self.reading {{
        self.cancel_active_read()
    }}
    self.closed = true
    wasmImport{symbol_name}DropReadable(self.handle)
}}

fn Wasm{symbol_name}FutureSource::close_sync(
    self : Wasm{symbol_name}FutureSource,
) -> Unit {{
    if self.closed || self.reading {{
        return
    }}
    {reject_prepared}
    self.closed = true
    wasmImport{symbol_name}DropReadable(self.handle)
}}

async fn Wasm{symbol_name}FutureSource::read(
    self : Wasm{symbol_name}FutureSource,
) -> {result} {{
    if self.closed {{
        raise {ffi}FutureReadError::Dropped
    }}
    if self.reading {{
        raise {ffi}EndpointBusy::Read
    }}
    let ptr = wasm{symbol_name}Malloc(1)
    defer {{ {free_outer} }}
    self.reading = true
    self.read_task = {ffi}current_component_task_token()
    self.read_buffer = ptr
    self.read_discarding = false
    self.read_cleanup_done = false
    {ffi}suspend_for_future_read(
        self.handle,
        wasmImport{symbol_name}Read(self.handle, ptr),
    ) catch {{
        err => {{
            if self.read_discarding {{
                self.wait_for_read_cleanup()
                self.finish_read()
                raise {ffi}FutureReadError::Dropped
            }}
            if err is {ffi}Cancelled::Cancelled {{
                self.cancel_active_read()
                if !self.closed {{
                    self.closed = true
                    wasmImport{symbol_name}DropReadable(self.handle)
                }}
            }}
            self.finish_read()
            raise err
        }}
    }}
    if self.read_discarding {{
        self.wait_for_read_cleanup()
        self.finish_read()
        raise {ffi}FutureReadError::Dropped
    }}
    let value = wasm{symbol_name}Lift(ptr)
    self.finish_read()
    if !self.closed {{
        self.closed = true
        wasmImport{symbol_name}DropReadable(self.handle)
    }}
    value
}}

fn wasm{symbol_name}FutureLift(handle : Int) -> {ffi}Future[{result}] {{
    let source = Wasm{symbol_name}FutureSource::{{
        handle,
        closed: false,
        reading: false,
        read_task: 0,
        read_buffer: 0,
        read_discarding: false,
        read_cleanup_done: false,
        read_cleanup: CondVar(),
    }}
    {ffi}Future::from_source(
        () => source.read(),
        () => source.close(),
        () => source.close_sync(),
    )
}}
"#
                        )
                    })
                    .unwrap_or_default();
                let lower_committed = endpoint_use
                    .lower_committed
                    .then(|| {
                        format!(
                            r#"
fn wasm{symbol_name}FutureLowerCommitted(future : {ffi}Future[{result}]) -> Int {{
    let reader = wasm{symbol_name}FutureLower(future)
    wasm{symbol_name}FutureCommit(reader)
    reader
}}
                            "#
                        )
                    })
                    .unwrap_or_default();
                let lower_bridge = if endpoint_use.lower {
                    format!(
                        r#"
priv struct Wasm{symbol_name}FutureProducer {{
    future : {ffi}Future[{result}]
    writer : Int
}}

let wasm{symbol_name}FutureProducers : Map[Int, Wasm{symbol_name}FutureProducer] = Map([])

{reject_prepared_func}

fn wasm{symbol_name}FutureCommit(handle : Int) -> Unit {{
    guard wasm{symbol_name}FutureProducers.get(handle) is Some(producer) else {{
        return
    }}
    wasm{symbol_name}FutureProducers.remove(handle)
    let writer = producer.writer
    if !{ffi}has_component_task_scope() {{
        abort("component future producer requires an async task scope")
    }}
    {ffi}spawn_component_task_current(async fn() {{
        {ffi}protect_from_cancel(
            () => {{
                let value = producer.future.get()
                let ptr = wasm{symbol_name}Malloc(1)
                wasm{symbol_name}Lower(value, ptr)
                let terminal : Ref[Bool?] = {{ val: None }}
                while terminal.val is None {{
                    terminal.val = {ffi}suspend_for_future_write_terminal(
                        writer,
                        wasmImport{symbol_name}Write(writer, ptr),
                    )
                }}
                guard terminal.val is Some(transferred)
                if transferred {{
                    wasm{symbol_name}Commit(ptr, 0, 1)
                }} else {{
                    wasm{symbol_name}Reject(ptr, 0, 1)
                }}
                {free_outer}
                wasmImport{symbol_name}DropWritable(writer)
            }},
            resume_on_cancel=true,
        ) catch {{
            _ => abort("component future producer ended without a value")
        }}
    }})
}}

fn wasm{symbol_name}FutureLower(future : {ffi}Future[{result}]) -> Int {{
    if !{ffi}has_component_task_scope() {{
        abort("component future producer requires an async task scope")
    }}
    let pair = wasmImport{symbol_name}New()
    let reader = pair.to_int()
    let writer = (pair >> 32).to_int()
    wasm{symbol_name}FutureProducers.set(reader, {{ future, writer }})
    reader
}}

{lower_committed}
"#
                    )
                } else {
                    String::new()
                };
                format!("{lift_bridge}{lower_bridge}")
            }
            PayloadFor::Stream => {
                let reject_prepared = (endpoint_use.lift && endpoint_use.lower)
                    .then(|| {
                        format!(
                            r#"
    if wasm{symbol_name}StreamRejectPrepared(self.handle) {{
        self.closed = true
        return
    }}
                            "#
                        )
                    })
                    .unwrap_or_default();
                let reject_prepared_func = (endpoint_use.lift && endpoint_use.lower)
                    .then(|| {
                        format!(
                            r#"
fn wasm{symbol_name}StreamRejectPrepared(handle : Int) -> Bool {{
    guard wasm{symbol_name}StreamProducers.get(handle) is Some(prepared) else {{
        return false
    }}
    wasm{symbol_name}StreamProducers.remove(handle)
    let writer = prepared.writer
    wasmImport{symbol_name}DropReadable(handle)
    wasmImport{symbol_name}DropWritable(writer)
    {ffi}spawn_component_task_current(async fn() {{
        {ffi}protect_from_cancel(
            () => {{
                prepared.stream.reject((value : {result}) => {{
                        let ptr = wasm{symbol_name}Malloc(1)
                        wasm{symbol_name}Lower(value, ptr)
                        wasm{symbol_name}Reject(ptr, 0, 1)
                        {free_outer}
                    }})
            }},
            resume_on_cancel=true,
        ) catch {{
            _ => ()
        }}
    }})
    true
}}
                            "#
                        )
                    })
                    .unwrap_or_default();
                let lift_bridge = endpoint_use
                    .lift
                    .then(|| {
                        format!(
                            r#"
priv struct Wasm{symbol_name}StreamSource {{
    handle : Int
    mut closed : Bool
    mut reading : Bool
    mut read_task : Int
    mut read_buffer : Int
    mut read_discarding : Bool
    mut read_cleanup_done : Bool
    read_cleanup : {ffi}CondVar
}}

fn Wasm{symbol_name}StreamSource::finish_read(
    self : Wasm{symbol_name}StreamSource,
) -> Unit {{
    self.reading = false
    self.read_task = 0
    self.read_buffer = 0
    self.read_discarding = false
    // Keep completion latched until a later read starts so every waiter woken
    // by the broadcast can observe it.
}}

async fn Wasm{symbol_name}StreamSource::wait_for_read_cleanup(
    self : Wasm{symbol_name}StreamSource,
) -> Unit noraise {{
    {ffi}protect_from_cancel(
        () => {{
            while !self.read_cleanup_done {{
                self.read_cleanup.wait()
            }}
        }},
        resume_on_cancel=true,
    ) catch {{
        _ => ()
    }}
}}

async fn Wasm{symbol_name}StreamSource::cancel_active_read(
    self : Wasm{symbol_name}StreamSource,
) -> Unit noraise {{
    if !self.reading {{
        return
    }}
    if self.read_discarding {{
        self.wait_for_read_cleanup()
        return
    }}
    self.read_discarding = true
    let progress_ref = Ref(0)
    {ffi}protect_from_cancel(
        () => {{
            progress_ref.val = {ffi}cancel_stream_read(
                self.read_task,
                self.handle,
                () => wasmImport{symbol_name}CancelRead(self.handle),
            )
        }},
        resume_on_cancel=true,
    ) catch {{
        _ => ()
    }}
    let progress = progress_ref.val
    if progress > 0 && self.read_buffer != 0 {{
        {reject_stream_read}
    }}
    self.read_cleanup_done = true
    self.read_cleanup.broadcast()
}}

async fn Wasm{symbol_name}StreamSource::close(
    self : Wasm{symbol_name}StreamSource,
) -> Unit noraise {{
    if self.closed {{
        return
    }}
    if self.reading {{
        self.cancel_active_read()
    }}
    self.closed = true
    wasmImport{symbol_name}DropReadable(self.handle)
}}

fn Wasm{symbol_name}StreamSource::close_sync(
    self : Wasm{symbol_name}StreamSource,
) -> Unit {{
    if self.closed || self.reading {{
        return
    }}
    {reject_prepared}
    self.closed = true
    wasmImport{symbol_name}DropReadable(self.handle)
}}

async fn Wasm{symbol_name}StreamSource::read(
    self : Wasm{symbol_name}StreamSource,
    count : Int,
) -> FixedArray[{result}]? {{
    if self.closed {{
        return None
    }}
    if count <= 0 {{
        return Some([])
    }}
    if self.reading {{
        raise {ffi}EndpointBusy::Read
    }}
    let ptr = wasm{symbol_name}Malloc(count)
    let mut owns_buffer = false
    defer {{
        if !owns_buffer {{
            {free_outer}
        }}
    }}
    self.reading = true
    self.read_task = {ffi}current_component_task_token()
    self.read_buffer = ptr
    self.read_discarding = false
    self.read_cleanup_done = false
    let (progress, end) = {ffi}suspend_for_stream_read(
        self.handle,
        wasmImport{symbol_name}Read(self.handle, ptr, count),
    ) catch {{
        err => {{
            if self.read_discarding {{
                self.wait_for_read_cleanup()
                self.finish_read()
                return None
            }}
            if err is {ffi}Cancelled::Cancelled {{
                self.cancel_active_read()
                if !self.closed {{
                    self.closed = true
                    wasmImport{symbol_name}DropReadable(self.handle)
                }}
            }}
            self.finish_read()
            raise err
        }}
    }}
    if self.read_discarding {{
        self.wait_for_read_cleanup()
        self.finish_read()
        return None
    }}
    if progress == 0 {{
        self.finish_read()
        if end {{
            self.close()
            return None
        }}
        return Some([])
    }}
    let values = wasm{symbol_name}ListLift(ptr, progress)
    owns_buffer = {read_chunk_owns_buffer}
    self.finish_read()
    if end {{
        self.close()
    }}
    Some(values)
}}

fn wasm{symbol_name}StreamLift(handle : Int) -> {ffi}Stream[{result}] {{
    let source = Wasm{symbol_name}StreamSource::{{
        handle,
        closed: false,
        reading: false,
        read_task: 0,
        read_buffer: 0,
        read_discarding: false,
        read_cleanup_done: false,
        read_cleanup: CondVar(),
    }}
    {ffi}Stream::from_source(
        (count) => source.read(count),
        () => source.close(),
        () => source.close_sync(),
    )
}}
"#
                        )
                    })
                    .unwrap_or_default();
                let lower_committed = endpoint_use
                    .lower_committed
                    .then(|| {
                        format!(
                            r#"
fn wasm{symbol_name}StreamLowerCommitted(stream : {ffi}Stream[{result}]) -> Int {{
    let reader = wasm{symbol_name}StreamLower(stream)
    wasm{symbol_name}StreamCommit(reader)
    reader
}}
                            "#
                        )
                    })
                    .unwrap_or_default();
                let lower_bridge = if endpoint_use.lower {
                    format!(
                        r#"
priv struct Wasm{symbol_name}StreamProducer {{
    stream : {ffi}Stream[{result}]
    writer : Int
}}

let wasm{symbol_name}StreamProducers : Map[Int, Wasm{symbol_name}StreamProducer] = Map([])

{reject_prepared_func}

fn wasm{symbol_name}StreamCommit(handle : Int) -> Unit {{
    guard wasm{symbol_name}StreamProducers.get(handle) is Some(prepared) else {{
        return
    }}
    wasm{symbol_name}StreamProducers.remove(handle)
    let stream = prepared.stream
    let writer = prepared.writer
    if !{ffi}has_component_task_scope() {{
        abort("component stream producer requires an async task scope")
    }}
    let producer = stream.take_producer()
    {ffi}spawn_component_task_current(async fn() {{
        let writer_closed = Ref(false)
        let writer_lock = {ffi}Mutex()
        let close_writer = fn() -> Unit {{
            if !writer_closed.val {{
                writer_closed.val = true
                wasmImport{symbol_name}DropWritable(writer)
            }}
        }}
        let close_writer_serialized = async fn() -> Unit {{
            writer_lock.acquire()
            defer writer_lock.release()
            close_writer()
        }}
        let cleanup_value = fn(value : {result}) -> Unit {{
            let ptr = wasm{symbol_name}Malloc(1)
            wasm{symbol_name}Lower(value, ptr)
            wasm{symbol_name}Reject(ptr, 0, 1)
            {free_outer}
        }}
        let sink = {ffi}Sink::from_callbacks(
            (data : ArrayView[{result}]) => {{
                writer_lock.acquire()
                defer writer_lock.release()
                if writer_closed.val || data.length() == 0 {{
                    return 0
                }}
                let data_len = if data.length() < {staging_window} {{
                    data.length()
                }} else {{
                    {staging_window}
                }}
                let ptr = wasm{symbol_name}Malloc(data_len)
                for i in 0..<data_len {{
                    let value : {result} = data[i]
                    wasm{symbol_name}Lower(value, ptr + i * {elem_size})
                }}
                let settle_staging = fn(transferred : Int) -> Unit {{
                    wasm{symbol_name}Commit(ptr, 0, transferred)
                    if transferred < data_len {{
                        wasm{symbol_name}Reject(
                            ptr,
                            transferred,
                            data_len - transferred,
                        )
                    }}
                    {free_outer}
                }}
                let mut total = 0
                let mut dropped = false
                while total < data_len {{
                    let (progress, end) = {ffi}suspend_for_stream_write(
                        writer,
                        wasmImport{symbol_name}Write(
                            writer,
                            ptr + total * {elem_size},
                            data_len - total,
                        ),
                    ) catch {{
                        _ => {{
                            total = total + {ffi}cancel_stream_write(
                                writer,
                                () => wasmImport{symbol_name}CancelWrite(writer),
                            )
                            settle_staging(total)
                            close_writer()
                            return data_len
                        }}
                    }}
                    total = total + progress
                    if end {{
                        dropped = true
                        break
                    }}
                }}
                settle_staging(total)
                if dropped {{
                    close_writer()
                }}
                // The canonical writer accepted `total`; generated cleanup
                // consumed the rest of this staging window.
                data_len
            }},
            () => close_writer_serialized(),
            () => !writer_closed.val,
            Some(cleanup_value),
        )
        let relay_source = producer is None
        let run_producer = async fn() -> Unit {{
            match producer {{
                Some(producer) => {{
                    producer(sink)
                    sink.close()
                }}
                None =>
                    for ;; {{
                        match stream.read({staging_window}) {{
                            Some(data) =>
                                if !sink.write_all(data[:]) {{
                                    stream.reject(cleanup_value)
                                    return
                                }}
                            None => {{
                                sink.close()
                                return
                            }}
                        }}
                    }}
            }}
        }}
        run_producer() catch {{
            _ =>
                if relay_source {{
                    {ffi}protect_from_cancel(
                        () => stream.reject(cleanup_value),
                        resume_on_cancel=true,
                    ) catch {{
                        _ => ()
                    }}
                }}
        }}
        // A retained Sink may still own an in-flight canonical write after its
        // producer returns. Do not drop the writable endpoint until that write
        // has reached a terminal event and released its staging buffer.
        {ffi}protect_from_cancel(
            () => close_writer_serialized(),
            resume_on_cancel=true,
        )
    }})
}}

fn wasm{symbol_name}StreamLower(stream : {ffi}Stream[{result}]) -> Int {{
    if !{ffi}has_component_task_scope() {{
        abort("component stream producer requires an async task scope")
    }}
    let pair = wasmImport{symbol_name}New()
    let reader = pair.to_int()
    let writer = (pair >> 32).to_int()
    wasm{symbol_name}StreamProducers.set(reader, {{ stream, writer }})
    reader
}}

{lower_committed}
"#
                    )
                } else {
                    String::new()
                };
                format!("{lift_bridge}{lower_bridge}")
            }
        };

        uwriteln!(
            self.ffi,
            r#"
{lift_intrinsics}
{drop_readable_intrinsic}
{lower_intrinsics}

fn wasm{symbol_name}Malloc(length : Int) -> Int {{
    {malloc}
    ptr
}}

{commit_func}

fn wasm{symbol_name}Reject(
    ptr : Int,
    start : Int,
    length : Int,
) -> Unit {{
    {reject}
}}

{lift_func}
{lower_func}
{list_lift_func}
{bridge_func}
"#,
        );
    }

    fn deallocate_lists_and_own(
        &mut self,
        types: &[Type],
        operands: &[String],
        indirect: bool,
        package: &str,
        endpoint_sites: Vec<AsyncEndpointSite>,
    ) -> String {
        self.deallocate_lists_and_own_with_state(types, operands, indirect, package, endpoint_sites)
            .0
    }

    fn deallocate_lists_and_own_with_state(
        &mut self,
        types: &[Type],
        operands: &[String],
        indirect: bool,
        package: &str,
        endpoint_sites: Vec<AsyncEndpointSite>,
    ) -> (String, AsyncFunctionState) {
        let mut f = FunctionBindgen::new(self, Box::new([]))
            .with_type_context(package)
            .with_async_state(AsyncFunctionState::from_sites(endpoint_sites))
            .with_sync_endpoint_drop();
        abi::deallocate_lists_and_own_in_types(
            f.interface_gen.resolve,
            types,
            operands,
            indirect,
            &mut f,
        );
        (f.src, f.async_state)
    }

    fn commit_lists_and_endpoints_with_state(
        &mut self,
        types: &[Type],
        operands: &[String],
        indirect: bool,
        package: &str,
        endpoint_sites: Vec<AsyncEndpointSite>,
    ) -> (String, AsyncFunctionState) {
        let mut f = FunctionBindgen::new(self, Box::new([]))
            .with_type_context(package)
            .with_async_state(AsyncFunctionState::from_sites(endpoint_sites))
            .with_endpoint_commit();
        abi::deallocate_lists_and_own_in_types(
            f.interface_gen.resolve,
            types,
            operands,
            indirect,
            &mut f,
        );
        (f.src, f.async_state)
    }

    fn lift_from_memory(
        &mut self,
        address: &str,
        ty: &Type,
        package: &str,
        async_state: &mut AsyncFunctionState,
    ) -> (String, String) {
        let mut f = FunctionBindgen::new(self, Box::new([]))
            .with_type_context(package)
            .with_async_state(mem::take(async_state));

        let result = abi::lift_from_memory(f.interface_gen.resolve, &mut f, address.into(), ty);
        *async_state = f.async_state;
        (f.src, result)
    }

    fn lower_to_memory(
        &mut self,
        address: &str,
        value: &str,
        ty: &Type,
        package: &str,
        async_state: &mut AsyncFunctionState,
    ) -> String {
        let mut f = FunctionBindgen::new(self, Box::new([address.to_string(), value.to_string()]))
            .with_type_context(package)
            .with_async_state(mem::take(async_state))
            .without_block_cleanup();
        abi::lower_to_memory(
            f.interface_gen.resolve,
            &mut f,
            address.into(),
            value.into(),
            ty,
        );
        *async_state = f.async_state;
        f.src
    }

    fn malloc_memory(&mut self, address: &str, length: &str, ty: &Type) -> String {
        let size = self.world_gen.sizes.size(ty).size_wasm32();
        self.ffi_imports.insert(ffi::MALLOC);
        format!("let {address} = mbt_ffi_malloc({size} * {length})")
    }

    fn is_list_canonical(&self, element: &Type) -> bool {
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
        if self.is_list_canonical(ty) {
            if ty == &Type::U8 {
                self.ffi_imports.insert(ffi::PTR2BYTES);
                return format!("mbt_ffi_ptr2bytes({address}, {length})");
            }
            let (ty, builtin) = match ty {
                Type::U32 => ("uint", ffi::PTR2UINT_ARRAY),
                Type::U64 => ("uint64", ffi::PTR2UINT64_ARRAY),
                Type::S32 => ("int", ffi::PTR2INT_ARRAY),
                Type::S64 => ("int64", ffi::PTR2INT64_ARRAY),
                Type::F32 => ("float", ffi::PTR2FLOAT_ARRAY),
                Type::F64 => ("double", ffi::PTR2DOUBLE_ARRAY),
                _ => unreachable!(),
            };

            self.ffi_imports.insert(builtin);
            return format!("mbt_ffi_ptr2{ty}_array({address}, {length})");
        }
        let size = self.world_gen.sizes.size(ty).size_wasm32();
        format!(
            r#"
            FixedArray::makei(
                {length},
                (index) => {{ 
                    let ptr = ({address}) + (index * {size})
                    {lift_func}(ptr)
                }}
            )
            "#
        )
    }
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    pub(super) fn with_type_context(mut self, type_context: &str) -> Self {
        self.type_context = type_context.to_string();
        self
    }

    pub(super) fn without_block_cleanup(mut self) -> Self {
        self.suppress_block_cleanup = true;
        self
    }

    pub(super) fn with_sync_endpoint_drop(mut self) -> Self {
        self.sync_endpoint_drop = true;
        self
    }

    pub(super) fn with_endpoint_commit(mut self) -> Self {
        self.commit_endpoints = true;
        self
    }

    pub(super) fn with_async_state(mut self, async_state: AsyncFunctionState) -> Self {
        self.async_state = async_state;
        self
    }

    pub(super) fn with_sync_import_commit(mut self, argument_types: Vec<Type>) -> Self {
        self.sync_import_argument_types = Some(argument_types);
        self
    }

    pub(super) fn commit_sync_import_arguments(
        &mut self,
        sig: &WasmSignature,
        operands: &[String],
    ) {
        let Some(argument_types) = self.sync_import_argument_types.clone() else {
            return;
        };
        if self.async_state.endpoint_sites.is_empty() {
            return;
        }

        let argument_count = if sig.indirect_params {
            1
        } else {
            operands.len() - usize::from(sig.retptr)
        };
        let argument_operands = operands[..argument_count].to_vec();
        let sites = self.async_state.endpoint_sites.clone();
        let previous_state =
            mem::replace(&mut self.async_state, AsyncFunctionState::from_sites(sites));
        let previous_commit_endpoints = mem::replace(&mut self.commit_endpoints, true);
        let previous_preserve_allocations =
            mem::replace(&mut self.preserve_guest_allocations, true);

        abi::deallocate_lists_and_own_in_types(
            self.interface_gen.resolve,
            &argument_types,
            &argument_operands,
            sig.indirect_params,
            self,
        );

        let commit_state = mem::replace(&mut self.async_state, previous_state);
        self.commit_endpoints = previous_commit_endpoints;
        self.preserve_guest_allocations = previous_preserve_allocations;
        for (used, committed) in self
            .async_state
            .endpoint_uses
            .iter_mut()
            .zip(commit_state.endpoint_uses)
        {
            used.lift |= committed.lift;
            used.lower |= committed.lower;
        }
    }

    pub(super) fn emit_async_call_interface(
        &mut self,
        func: &Function,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        let name = self.interface_gen.world_gen.pkg_resolver.func_call(
            &self.type_context,
            func,
            &self.func_interface,
        );
        let task_return_name = format!("{}_task_return", func.name.to_moonbit_ident());

        let mut args = operands.to_vec();
        if matches!(self.interface_gen.direction, Direction::Export) {
            args.push(background_group_name(func));
        }
        let args = args.join(", ");
        let (return_param, return_value) = match func.result {
            Some(ty) => {
                let return_param = self.locals.tmp("return_result");
                let return_value = self.locals.tmp("return_value");
                let task_return_type = self
                    .interface_gen
                    .world_gen
                    .pkg_resolver
                    .type_name(&self.type_context, &ty);
                results.push(return_value.clone());
                uwrite!(
                    self.src,
                    r#"
                    let {return_param}: Ref[{task_return_type}?] = Ref(None)
                    {return_param}.val = Some({name}({args}))
                    {task_return_name}({return_param})
                    "#,
                );
                (return_param, return_value)
            }
            None => {
                uwrite!(
                    self.src,
                    r#"
                    {name}({args})
                    {task_return_name}()
                    "#,
                );
                (String::new(), String::new())
            }
        };
        assert!(matches!(
            self.async_state.task_return,
            AsyncTaskReturnState::None
        ));
        self.async_state.task_return = AsyncTaskReturnState::Generating {
            prev_src: mem::take(&mut self.src),
            prev_needs_cleanup_list: mem::replace(&mut self.needs_cleanup_list, false),
            return_param,
            return_value,
        };
    }

    fn emit_endpoint_lift(
        &mut self,
        kind: PayloadFor,
        ty: TypeId,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        let result = self.locals.tmp("result");
        let op = &operands[0];
        let site = self
            .async_state
            .next_site(kind, ty, EndpointOperation::Lift);
        let type_name = kind.type_name();
        let lift_name = format!("wasm{}{type_name}Lift", site.symbol_name);

        if self.commit_endpoints {
            let commit_name = format!("wasm{}{type_name}Commit", site.symbol_name);
            uwriteln!(self.src, r#"{commit_name}({op})"#);
            results.push("()".into());
            return;
        }

        uwriteln!(self.src, r#"let {result} = {lift_name}({op})"#,);

        results.push(result);
    }

    fn emit_endpoint_lower(
        &mut self,
        kind: PayloadFor,
        ty: TypeId,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        let committed = matches!(
            self.async_state.task_return,
            AsyncTaskReturnState::Generating { .. }
        );
        let operation = if committed {
            EndpointOperation::LowerCommitted
        } else {
            EndpointOperation::Lower
        };
        let site = self.async_state.next_site(kind, ty, operation);
        let type_name = kind.type_name();
        let suffix = if committed { "LowerCommitted" } else { "Lower" };
        let lower_name = format!("wasm{}{type_name}{suffix}", site.symbol_name);
        let op = &operands[0];
        results.push(format!("{lower_name}({op})"));
    }

    pub(super) fn emit_future_lift(
        &mut self,
        ty: TypeId,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        self.emit_endpoint_lift(PayloadFor::Future, ty, operands, results);
    }

    pub(super) fn emit_future_lower(
        &mut self,
        ty: TypeId,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        self.emit_endpoint_lower(PayloadFor::Future, ty, operands, results);
    }

    pub(super) fn capture_task_return(&mut self, params: &[WasmType], operands: &[String]) {
        let (body, needs_cleanup_list, return_param, return_value) =
            match &mut self.async_state.task_return {
                AsyncTaskReturnState::Generating {
                    prev_src,
                    prev_needs_cleanup_list,
                    return_param,
                    return_value,
                } => {
                    mem::swap(&mut self.src, prev_src);
                    let needs_cleanup_list =
                        mem::replace(&mut self.needs_cleanup_list, *prev_needs_cleanup_list);
                    (
                        mem::take(prev_src),
                        needs_cleanup_list,
                        return_param.clone(),
                        return_value.clone(),
                    )
                }
                _ => unreachable!(),
            };
        assert_eq!(params.len(), operands.len());
        self.async_state.task_return = AsyncTaskReturnState::Emitted {
            body,
            needs_cleanup_list,
            params: params
                .iter()
                .zip(operands)
                .map(|(a, b)| (*a, b.clone()))
                .collect(),
            return_param,
            return_value,
        };
    }

    pub(super) fn emit_stream_lower(
        &mut self,
        ty: TypeId,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        self.emit_endpoint_lower(PayloadFor::Stream, ty, operands, results);
    }

    pub(super) fn emit_stream_lift(
        &mut self,
        ty: TypeId,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        self.emit_endpoint_lift(PayloadFor::Stream, ty, operands, results);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wit_bindgen_core::wit_parser::{Docs, FunctionKind, Stability, TypeDef, TypeOwner};

    fn future_type(resolve: &mut Resolve, payload: Option<Type>) -> TypeId {
        resolve.types.alloc(TypeDef {
            name: None,
            kind: TypeDefKind::Future(payload),
            owner: TypeOwner::None,
            docs: Docs::default(),
            stability: Stability::Unknown,
            span: Default::default(),
            external_id: None,
        })
    }

    fn test_function(params: Vec<(&str, Type)>, result: Option<Type>) -> Function {
        Function {
            name: "f".into(),
            kind: FunctionKind::Freestanding,
            params: params
                .into_iter()
                .map(|(name, ty)| Param {
                    name: name.to_string(),
                    ty,
                    span: Default::default(),
                })
                .collect(),
            result,
            docs: Docs::default(),
            stability: Stability::Unknown,
            span: Default::default(),
            external_id: None,
        }
    }

    #[test]
    fn duplicate_future_sites_have_distinct_symbols() {
        let mut resolve = Resolve::default();
        let future = future_type(&mut resolve, Some(Type::U32));
        let func = test_function(
            vec![("a", Type::Id(future)), ("b", Type::Id(future))],
            Some(Type::Id(future)),
        );

        let names = AsyncEndpointNames::import(&resolve, None, &func);
        let sites = async_endpoint_sites(&resolve, &names, None, &func, false);
        assert_eq!(sites.len(), 3);
        assert_eq!(sites[0].ty, future);
        assert_eq!(sites[1].ty, future);
        assert_eq!(sites[2].ty, future);
        assert_ne!(sites[0].symbol_name, sites[1].symbol_name);
        assert_ne!(sites[1].symbol_name, sites[2].symbol_name);

        let first_symbol = sites[0].symbol_name.clone();
        let second_symbol = sites[1].symbol_name.clone();
        let third_symbol = sites[2].symbol_name.clone();
        let mut state = AsyncFunctionState::from_sites(sites);

        assert_eq!(
            state
                .next_site(PayloadFor::Future, future, EndpointOperation::Lower)
                .symbol_name,
            first_symbol
        );
        assert_eq!(
            state
                .next_site(PayloadFor::Future, future, EndpointOperation::Lower)
                .symbol_name,
            second_symbol
        );
        assert_eq!(
            state
                .next_site(PayloadFor::Future, future, EndpointOperation::Lower)
                .symbol_name,
            third_symbol
        );
    }

    #[test]
    fn nested_payload_sites_keep_their_original_indices() {
        let mut resolve = Resolve::default();
        let inner = future_type(&mut resolve, Some(Type::U32));
        let outer = future_type(&mut resolve, Some(Type::Id(inner)));
        let func = test_function(vec![("a", Type::Id(outer)), ("b", Type::Id(outer))], None);

        let names = AsyncEndpointNames::import(&resolve, None, &func);
        let sites = async_endpoint_sites(&resolve, &names, None, &func, false);
        assert_eq!(
            sites.iter().map(|site| site.index).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            sites.iter().map(|site| site.ty).collect::<Vec<_>>(),
            vec![inner, outer, inner, outer]
        );

        let plan = AsyncFunctionPlan::new(sites.clone());
        let first_outer_payload = plan.payload_sites(&sites[1]);
        let second_outer_payload = plan.payload_sites(&sites[3]);

        assert_eq!(first_outer_payload[0].index, 0);
        assert_eq!(second_outer_payload[0].index, 2);

        let mut state = AsyncFunctionState::from_sites(second_outer_payload);
        assert_eq!(
            state
                .next_site(PayloadFor::Future, inner, EndpointOperation::Lift)
                .symbol_name,
            sites[2].symbol_name
        );
    }
}
