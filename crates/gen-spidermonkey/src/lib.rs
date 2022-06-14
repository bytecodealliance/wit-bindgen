//! Interface types bindings generator support for `spidermonkey.wasm`.

#![deny(missing_docs)]

mod data_segments;

use data_segments::DataSegments;
use heck::SnakeCase;
use lazy_static::lazy_static;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::ops::Range;
use std::path::PathBuf;
use std::{collections::HashMap, mem};
use wasm_encoder::Instruction;
use wit_bindgen_gen_core::{
    wit_parser::{
        abi::{self, AbiVariant, WasmSignature, WasmType},
        Docs, Enum, Expected, Flags, Function, Interface, Record, ResourceId, SizeAlign, Tuple,
        Type, TypeId, Union, Variant,
    },
    Direction, Files, Generator,
};

#[allow(missing_docs)]
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    /// The path to the JavaScript module.
    pub js: PathBuf,
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub import_spidermonkey: bool,
}

#[allow(missing_docs)]
impl Opts {
    pub fn build<'a>(self, js_source: impl Into<Cow<'a, str>>) -> SpiderMonkeyWasm<'a> {
        let mut builder = SpiderMonkeyWasm::new(self.js, js_source);
        builder.import_spidermonkey(self.import_spidermonkey);
        builder
    }
}

lazy_static! {
    /// Functions exported from `spidermonkey.wasm`
    static ref SMW_EXPORTS: Vec<(&'static str, WasmSignature)> = vec![
        (
            "_initialize",
            WasmSignature {
                params: vec![],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "canonical_abi_free",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "canonical_abi_realloc",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32],
                results: vec![WasmType::I32],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_initialize_engine",
            WasmSignature {
                params: vec![],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_new_module_builder",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32],
                results: vec![WasmType::I32],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_module_builder_add_export",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_finish_module_builder",
            WasmSignature {
                params: vec![WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_eval_module",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_malloc",
            WasmSignature {
                params: vec![WasmType::I32],
                results: vec![WasmType::I32],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_fill_operands",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
    }
        ),
        (
            "SMW_clear_operands",
            WasmSignature {
                params: vec![],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_push_arg",
            WasmSignature {
                params: vec![WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_call",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32, WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_push_return_value",
            WasmSignature {
                params: vec![WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_finish_returns",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_i32_from_u32",
            WasmSignature {
                params: vec![WasmType::I32],
                results: vec![WasmType::I32],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_u32_from_i32",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_string_canon_lower",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_string_canon_lift",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_spread_into_array",
            WasmSignature {
                params: vec![WasmType::I32],
                results: vec![WasmType::I32],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_get_array_element",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_array_push",
            WasmSignature {
                params: vec![WasmType::I32, WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "SMW_new_array",
            WasmSignature {
                params: vec![WasmType::I32],
                results: vec![],
                retptr: false,
                indirect_params: false,
            },
        ),
        (
            "dump_i32",
            WasmSignature {
                params: vec![WasmType::I32],
                results: vec![WasmType::I32],
                retptr: false,
                indirect_params: false,
            },
        ),
    ];
}

/// The `spidermonkey.wasm` bindings generator.
///
/// ## Code Shape
///
/// The output is a single Wasm file that imports and exports the functions
/// defined in the given WIT files and additionally
///
/// * embeds or imports (configurable) a `spidermonkey.wasm` instance, and
///
/// * exports a `wizer.initialize` function that initializes SpiderMonkey and
///   evaluates the top level of the JavaScript.
///
/// ### Initialization
///
/// As an API contract, the `wizer.initialize` function must be invoked before
/// any other function. It must only be invoked once.
///
/// The initialization function performs the following tasks:
///
/// * Calls `spidermonkey.wasm`'s `_initialize` function, which runs C++ global
///   contructors.
///
/// * `malloc`s space in `spidermonkey.wasm`'s linear memory and copies the
///   JavaScript source code from its linear memory into the malloc'd space.
///
/// * Evaluates the JavaScript source, compiling it to bytecode and initializing
///   globals and defining top-level functions in the process.
///
/// ### Imports
///
/// By the time an imported WIT function is called, we have the following
/// layers of code on the stack, listed from older to younger frames:
///
/// * User JS code (inside `spidermonkey.wasm`'s internal JS stack)
///
///   This is the user's JavaScript code that is running inside of
///   `spidermonkey.wasm` and which wants to call an external, imported function
///   that is described with WIT.
///
/// * Import glue Wasm code (on the Wasm stack)
///
///   This is a synthesized Wasm function that understands both the canonical
///   ABI and the SpiderMonkey API. It translates outgoing arguments from
///   SpiderMonkey values into the canonical ABI representation, calls the
///   actual imported Wasm function, and then translates the incoming results
///   from the canonical ABI representation into SpiderMonkey values.
///
/// * Imported function (on the Wasm Stack)
///
///   This is the actual Wasm function whose signature is described in WIT and
///   uses the canonical ABI.
///
/// ### Exports
///
/// By the time an exported JS function that implements a WIT signature is
/// called, we have the following frames on the stack, listed form older to
/// younger frames:
///
/// * External caller (on the Wasm or native stack)
///
///   This is whoever is calling our JS-implemented WIT export, using the
///   canonical ABI. This might be another Wasm module or it might be some
///   native code in the host.
///
/// * Export glue Wasm code (on the Wasm stack)
///
///   This is a synthesized function that understands both the canonical ABI and
///   the SpiderMonkey API. It translates incoming arguments from the canonical
///   ABI representation into SpiderMonkey values, calls the JS function that
///   implements this export with those values, and then translates the
///   function's outgoing results from SpiderMonkey values into the canonical
///   ABI representation.
///
/// * JavaScript function implementing the WIT signature (inside
///   `spidermonkey.wasm`'s internal stack)
///
///   This is the user-written JavaScript function that is being exported. It
///   accepts and returns the JavaScript values that correspond to the interface
///   types used in the WIT signature.
pub struct SpiderMonkeyWasm<'a> {
    /// The filename to use for the JS.
    js_name: PathBuf,

    /// The JS source code.
    js: Cow<'a, str>,

    return_pointer_area_size: usize,
    return_pointer_area_align: usize,

    num_import_functions: Option<u32>,
    num_export_functions: Option<u32>,

    import_spidermonkey: bool,

    /// Function types that we use in this Wasm module.
    types: wasm_encoder::TypeSection,

    /// A map from wasm signature to its index in the `self.types` types
    /// section. We use this to reuse earlier type definitions when possible.
    wasm_sig_to_index: HashMap<WasmSignature, u32>,

    /// The imports section containing the raw canonical ABI function imports
    /// for each imported function we are wrapping in glue.
    imports: wasm_encoder::ImportSection,

    /// The glue functions we've generated for imported canonical ABI functions
    /// thus far.
    import_glue_fns: Vec<wasm_encoder::Function>,

    /// A map from `module_name -> func_name -> (index, num_args)`.
    import_fn_name_to_index: HashMap<String, HashMap<String, (u32, u32)>>,

    exports: wasm_encoder::ExportSection,

    /// The glue functions we've generated for exported canonical ABI functions
    /// thus far, and their type index.
    export_glue_fns: Vec<(wasm_encoder::Function, u32)>,

    data_segments: DataSegments,

    sizes: SizeAlign,
    function_names: Vec<(u32, String)>,
    local_names: Vec<(u32, wasm_encoder::NameMap)>,
}

impl<'a> SpiderMonkeyWasm<'a> {
    /// Construct a new `SpiderMonkeyWasm` bindings generator using the given
    /// JavaScript module.
    pub fn new(js_name: impl Into<PathBuf>, js: impl Into<Cow<'a, str>>) -> Self {
        let js_name = js_name.into();
        let js = js.into();
        SpiderMonkeyWasm {
            js_name,
            js,
            return_pointer_area_size: 0,
            return_pointer_area_align: 0,
            num_import_functions: None,
            num_export_functions: None,
            import_spidermonkey: false,
            types: wasm_encoder::TypeSection::new(),
            wasm_sig_to_index: Default::default(),
            imports: wasm_encoder::ImportSection::new(),
            import_glue_fns: Default::default(),
            import_fn_name_to_index: Default::default(),
            exports: wasm_encoder::ExportSection::new(),
            export_glue_fns: Default::default(),
            data_segments: DataSegments::new(1),
            sizes: Default::default(),
            function_names: Vec::new(),
            local_names: Vec::new(),
        }
    }

    /// Configure how `spidermonkey.wasm` is linked.
    ///
    /// By default, the whole `spidermonkey.wasm` module is embedded inside our
    /// generated glue module's `module` section, and then instantiated in the
    /// `instance` section.
    ///
    /// If `import` is `true`, then `spidermonkey.wasm` is not embedded into the
    /// generated glue module. Instead, the glue module imports a
    /// `spidermonkey.wasm` instance.
    pub fn import_spidermonkey(&mut self, import: bool) {
        self.import_spidermonkey = import;
    }

    fn intern_type(&mut self, wasm_sig: WasmSignature) -> u32 {
        if let Some(idx) = self.wasm_sig_to_index.get(&&wasm_sig) {
            return *idx;
        }

        let idx = self.types.len();

        self.types.function(
            wasm_sig.params.iter().copied().map(convert_ty),
            wasm_sig.results.iter().copied().map(convert_ty),
        );

        self.wasm_sig_to_index.insert(wasm_sig, idx);

        idx
    }

    fn link_spidermonkey_wasm(
        &mut self,
        modules: &mut wasm_encoder::ModuleSection,
        instances: &mut wasm_encoder::InstanceSection,
        aliases: &mut wasm_encoder::AliasSection,
    ) {
        if self.import_spidermonkey {
            // Import an instance that exports all the expected
            // `spidermonkey.wasm` things.
            let exports: Vec<_> = SMW_EXPORTS
                .iter()
                .map(|(name, sig)| {
                    let idx = self.intern_type(sig.clone());
                    (*name, wasm_encoder::EntityType::Function(idx))
                })
                .chain(Some((
                    "memory",
                    wasm_encoder::EntityType::Memory(wasm_encoder::MemoryType {
                        minimum: 0,
                        maximum: None,
                        memory64: false,
                    }),
                )))
                .chain(Some((
                    "__indirect_function_table",
                    wasm_encoder::EntityType::Table(wasm_encoder::TableType {
                        element_type: wasm_encoder::ValType::FuncRef,
                        minimum: 0,
                        maximum: None,
                    }),
                )))
                .collect();
            let instance_type_index = self.types.len();
            self.types.instance(exports);
            self.imports.import(
                "spidermonkey",
                None,
                wasm_encoder::EntityType::Instance(instance_type_index),
            );
        } else {
            // Embded `spidermonkey.wasm` in the modules section and then
            // instantiate it. This will involve adding its imports to our
            // import section and fowarding them along.
            let _ = (modules, instances);
            todo!()
        }

        // Regardless whether we imported an instance or instantiated an embedded
        // module, we now have an instance of `spidermonkey.wasm`. Alias its
        // exported functions and exported memory into this module's index
        // spaces.
        let instance_index = u32::try_from(self.import_fn_name_to_index.len()).unwrap();
        aliases.instance_export(instance_index, wasm_encoder::ItemKind::Memory, "memory");
        aliases.instance_export(
            instance_index,
            wasm_encoder::ItemKind::Table,
            "__indirect_function_table",
        );
        for (name, _) in &*SMW_EXPORTS {
            aliases.instance_export(instance_index, wasm_encoder::ItemKind::Function, name);
            let idx = self.spidermonkey_import(name);
            self.function_names.push((idx, name.to_string()));
        }
    }

    /// Malloc `size` bytes and save the result to `local`.
    ///
    /// Note that `SMW_malloc` will never return `nullptr`.
    ///
    /// ```wat
    /// (local.set ${local} (call $SMW_malloc (i32.const ${size})))
    /// ```
    fn malloc_static_size<'b, F>(&mut self, func: &mut F, size: u32, result_local: u32)
    where
        F: InstructionSink<'b>,
    {
        // []
        func.instruction(Instruction::I32Const(size as _));
        // [i32]
        func.instruction(Instruction::Call(self.spidermonkey_import("SMW_malloc")));
        // [i32]
        func.instruction(Instruction::LocalSet(result_local));
        // []
    }

    /// Malloc `size` bytes and save the result to `local`. Trap if `malloc`
    /// returned `nullptr`.
    ///
    /// Note that `SMW_malloc` will never return `nullptr`.
    ///
    /// ```wat
    /// (local.set ${result_local} (call $malloc (local.get ${size_local})))
    /// ```
    fn malloc_dynamic_size<'b, F>(&mut self, func: &mut F, size_local: u32, result_local: u32)
    where
        F: InstructionSink<'b>,
    {
        // []
        func.instruction(Instruction::LocalGet(size_local));
        // [i32]
        func.instruction(Instruction::Call(self.spidermonkey_import("SMW_malloc")));
        // [i32]
        func.instruction(Instruction::LocalSet(result_local));
        // []
    }

    /// Copy data from the root glue module's linear memory into
    /// `spidermonkey.wasm`'s linear memory:
    ///
    /// ```wat
    /// (memory.copy 0 1 (local.get ${to_local})
    ///                  (i32.const ${from_offset})
    ///                  (i32.const ${len}))
    /// ```
    fn copy_to_smw<'b, F>(&self, func: &mut F, from_offset: u32, to_local: u32, len: u32)
    where
        F: InstructionSink<'b>,
    {
        // []
        func.instruction(Instruction::LocalGet(to_local));
        // [i32]
        func.instruction(Instruction::I32Const(from_offset as _));
        // [i32 i32]
        func.instruction(Instruction::I32Const(len as _));
        // [i32 i32 i32]
        func.instruction(Instruction::MemoryCopy {
            src: GLUE_MEMORY,
            dst: SM_MEMORY,
        });
        // []
    }

    fn clear_js_operands<'b, F>(&self, func: &mut F)
    where
        F: InstructionSink<'b>,
    {
        // []
        func.instruction(Instruction::Call(
            self.spidermonkey_import("SMW_clear_operands"),
        ));
        // []
    }

    fn define_wizer_initialize(
        &mut self,
        funcs: &mut wasm_encoder::FunctionSection,
        code: &mut wasm_encoder::CodeSection,
        js_name_offset: u32,
        js_name_len: u32,
        js_offset: u32,
        js_len: u32,
    ) {
        assert_eq!(funcs.len(), code.len());

        let wizer_init_index = self.wit_import_functions_len()
            + u32::try_from(SMW_EXPORTS.len()).unwrap()
            + funcs.len();
        self.function_names
            .push((wizer_init_index, format!("wizer.initialize")));

        let ty_index = self.intern_type(WasmSignature {
            params: vec![],
            results: vec![],
            retptr: false,
            indirect_params: false,
        });
        funcs.function(ty_index);

        let locals = vec![(7, wasm_encoder::ValType::I32)];
        let js_name_local = 0;
        let js_local = 1;
        let module_name_local = 2;
        let module_builder_local = 3;
        let table_size_local = 4;
        let func_name_local = 5;
        let ret_ptr_local = 6;

        let mut local_names = wasm_encoder::NameMap::new();
        local_names.append(js_name_local, "js_name");
        local_names.append(js_local, "js");
        local_names.append(module_name_local, "module_name");
        local_names.append(module_builder_local, "module_builder");
        local_names.append(table_size_local, "table_size");
        local_names.append(func_name_local, "func_name");
        local_names.append(ret_ptr_local, "ret_ptr");
        self.local_names.push((wizer_init_index, local_names));

        let mut wizer_init = wasm_encoder::Function::new(locals);

        // Call `_initialize` because that must be called before any other
        // exports per the WASI reactor ABI.
        let init_index = self.spidermonkey_import("_initialize");
        wizer_init.instruction(&Instruction::Call(init_index));

        // Malloc space in `spidermonkey.wasm`'s linear memory for the JS file
        // name and the JS source.
        self.malloc_static_size(&mut wizer_init, js_name_len, js_name_local);
        self.malloc_static_size(&mut wizer_init, js_len, js_local);

        // Copy the data into the freshly allocated regions.
        self.copy_to_smw(&mut wizer_init, js_name_offset, js_name_local, js_name_len);
        self.copy_to_smw(&mut wizer_init, js_offset, js_local, js_len);

        // Allocate space in the `spidermonkey.wasm` memory for the return
        // pointer area and save it to the return pointer global.
        //
        // TODO: handle `self.return_pointer_area_align` here
        if self.return_pointer_area_size > 0 {
            self.malloc_static_size(
                &mut wizer_init,
                u32::try_from(self.return_pointer_area_size).unwrap(),
                ret_ptr_local,
            );
            // []
            wizer_init.instruction(&Instruction::LocalGet(ret_ptr_local));
            // [i32]
            wizer_init.instruction(&Instruction::GlobalSet(RET_PTR_GLOBAL));
            // []
        }

        // Call `SMW_initialize_engine`:
        //
        //     (call $SMW_initialize_engine)
        let smw_initialize_engine = self.spidermonkey_import("SMW_initialize_engine");
        wizer_init.instruction(&Instruction::Call(smw_initialize_engine));

        // Define a JS module for each WIT module that is imported. This JS
        // module will export each of our generated glue functions for that WIT
        // module.
        let smw_new_module_builder = self.spidermonkey_import("SMW_new_module_builder");
        let import_fn_name_to_index =
            std::mem::replace(&mut self.import_fn_name_to_index, Default::default());
        for (module, funcs) in &import_fn_name_to_index {
            // Malloc space for the module name.
            self.malloc_static_size(
                &mut wizer_init,
                u32::try_from(module.len()).unwrap(),
                module_name_local,
            );

            // Copy the module name into the malloc'd space.
            let module_offset = self.data_segments.add(module.as_bytes().iter().copied());
            self.copy_to_smw(
                &mut wizer_init,
                module_offset,
                module_name_local,
                u32::try_from(module.len()).unwrap(),
            );

            // Call `SMW_new_module_builder`, passing it the module name:
            //
            //     (call $SMW_new_module_builder (local.get ${module_name})
            //                                   (i32.const ${module.len()}))
            //     local.set ${module_builder}
            wizer_init
                // []
                .instruction(&Instruction::LocalGet(module_name_local))
                // [i32]
                .instruction(&Instruction::I32Const(i32::try_from(module.len()).unwrap()))
                // [i32 i32]
                .instruction(&Instruction::Call(smw_new_module_builder))
                // [i32]
                .instruction(&Instruction::LocalSet(module_builder_local));
            // []

            // Grow enough space in the function table for the functions we will
            // add to it. Check for failure to allocate and trap if so.
            //
            //     (table.grow (ref.null) (i32.const ${funcs.len()}))
            //     local.tee ${table_size}
            //     i32.const -1
            //     i32.eq
            //     if
            //       unreachable
            //     end
            wizer_init
                // []
                .instruction(&Instruction::RefNull(wasm_encoder::ValType::FuncRef))
                // [funcref]
                .instruction(&Instruction::I32Const(i32::try_from(funcs.len()).unwrap()))
                // [funcref i32]
                .instruction(&Instruction::TableGrow { table: 0 })
                // [i32]
                .instruction(&Instruction::LocalTee(table_size_local))
                // [i32]
                .instruction(&Instruction::I32Const(-1))
                // [i32 i32]
                .instruction(&Instruction::I32Eq)
                // [i32]
                .instruction(&Instruction::If(wasm_encoder::BlockType::Empty))
                // []
                .instruction(&Instruction::Unreachable)
                // []
                .instruction(&Instruction::End);
            // []

            for (i, (func, (func_index, num_args))) in funcs.iter().enumerate() {
                // Malloc space for the function's name.
                self.malloc_static_size(
                    &mut wizer_init,
                    u32::try_from(func.len()).unwrap(),
                    func_name_local,
                );

                // Copy the function's name into the malloc'd space.
                let func_name_offset = self.data_segments.add(func.as_bytes().iter().copied());
                self.copy_to_smw(
                    &mut wizer_init,
                    func_name_offset,
                    func_name_local,
                    u32::try_from(func.len()).unwrap(),
                );

                // Set `table[orig_size + i]` to our synthesized import glue
                // function:
                //
                //     (table.set (i32.add (i32.const ${i}) (local.get ${table_size}))
                //                (ref.func ${func_index}))
                let glue_func_index = self.wit_import_glue_fn(*func_index);
                wizer_init
                    // []
                    .instruction(&Instruction::I32Const(i32::try_from(i).unwrap()))
                    // [i32]
                    .instruction(&Instruction::LocalGet(table_size_local))
                    // [i32 i32]
                    .instruction(&Instruction::I32Add)
                    // [i32]
                    .instruction(&Instruction::RefFunc(glue_func_index))
                    // [i32 funcref]
                    .instruction(&Instruction::TableSet { table: 0 });
                // []

                // Call `SMW_module_builder_add_export` passing the index of the
                // function that we just inserted into the table:
                //
                //     (call $SMW_module_builder_add_export (local.get ${module_builder})
                //                                          (local.get ${func_name})
                //                                          (i32.const ${func.len()})
                //                                          (i32.add (i32.const ${i}) (local.get ${table_size}))
                //                                          (i32.const ${num_args}))
                let smw_module_builder_add_export =
                    self.spidermonkey_import("SMW_module_builder_add_export");
                wizer_init
                    // []
                    .instruction(&Instruction::LocalGet(module_builder_local))
                    // [i32]
                    .instruction(&Instruction::LocalGet(func_name_local))
                    // [i32 i32]
                    .instruction(&Instruction::I32Const(i32::try_from(func.len()).unwrap()))
                    // [i32 i32 i32]
                    .instruction(&Instruction::I32Const(i32::try_from(i).unwrap()))
                    // [i32 i32 i32 i32]
                    .instruction(&Instruction::LocalGet(table_size_local))
                    // [i32 i32 i32 i32 i32]
                    .instruction(&Instruction::I32Add)
                    // [i32 i32 i32 i32]
                    .instruction(&Instruction::I32Const(i32::try_from(*num_args).unwrap()))
                    // [i32 i32 i32 i32 i32]
                    .instruction(&Instruction::Call(smw_module_builder_add_export));
                // []
            }

            // Call `SMW_finish_module_builder` to register the module:
            //
            //     (call $SMW_finish_module_builder (local.get ${module_builder}))
            let smw_finish_module_builder = self.spidermonkey_import("SMW_finish_module_builder");
            wizer_init
                // []
                .instruction(&Instruction::LocalGet(module_builder_local))
                // [i32]
                .instruction(&Instruction::Call(smw_finish_module_builder));
            // []
        }

        // Call `SMW_eval_module`, passing it the pointers to the JS file name
        // and JS source:
        //
        //     (call $SMW_eval_module (local.get 0) (local.get 1) (i32.const ${js_len}))
        let smw_eval_module = self.spidermonkey_import("SMW_eval_module");
        wizer_init
            // []
            .instruction(&Instruction::LocalGet(js_name_local))
            // [i32]
            .instruction(&Instruction::LocalGet(js_local))
            // [i32 i32]
            .instruction(&Instruction::I32Const(js_len as i32))
            // [i32 i32 i32]
            .instruction(&Instruction::Call(smw_eval_module));
        // []

        wizer_init.instruction(&Instruction::End);
        code.function(&wizer_init);

        self.exports.export(
            "wizer.initialize",
            wasm_encoder::Export::Function(wizer_init_index),
        );
    }
}

// ### Function Index Space
//
// The generated glue module's function index space is laid out as follows:
//
// ```text
// |wit imports...|spidermonkey.wasm imports...|import glue...|export glue...|wizer.initialize|
// ```
impl SpiderMonkeyWasm<'_> {
    /// Get the number of imported WIT functions.
    fn wit_import_functions_len(&self) -> u32 {
        self.num_import_functions
            .expect("must call `preprocess_all` before generating bindings")
    }

    /// Get the function index for the i^th WIT import.
    fn wit_import(&self, i: u32) -> u32 {
        i
    }

    /// Get the function index for the given spidermonkey function.
    fn spidermonkey_import(&self, name: &str) -> u32 {
        self.wit_import_functions_len()
            + u32::try_from(
                SMW_EXPORTS
                    .iter()
                    .position(|(n, _)| *n == name)
                    .unwrap_or_else(|| panic!("unknown `spidermonkey.wasm` export: {}", name)),
            )
            .unwrap()
    }

    /// Get the function index where WIT import glue functions start.
    fn wit_import_glue_fns_start(&self) -> u32 {
        self.wit_import_functions_len() + u32::try_from(SMW_EXPORTS.len()).unwrap()
    }

    /// Get the range of indices for our synthesized glue functions for WIT
    /// imports.
    fn wit_import_glue_fn_range(&self) -> Range<u32> {
        let start = self.wit_import_glue_fns_start();
        let end = self.wit_export_start();
        start..end
    }

    /// Get the function index for the i^th synthesized glue function for a WIT
    /// import.
    fn wit_import_glue_fn(&self, i: u32) -> u32 {
        assert!(
            i < self.wit_import_functions_len(),
            "{} < {}",
            i,
            self.wit_import_functions_len()
        );
        let start = self.wit_import_glue_fns_start();
        start + i
    }

    /// Get the function index where WIT export glue functions start.
    fn wit_export_start(&self) -> u32 {
        self.wit_import_glue_fns_start() + self.wit_import_functions_len()
    }

    fn wit_exports_len(&self) -> u32 {
        self.num_export_functions
            .expect("must call `preprocess_all` before generating bindings")
    }

    /// Get the function index for the i^th WIT export.
    fn wit_export(&self, i: u32) -> u32 {
        assert!(i < self.wit_exports_len());
        self.wit_export_start() + i
    }
}

impl Generator for SpiderMonkeyWasm<'_> {
    fn preprocess_all(&mut self, imports: &[Interface], exports: &[Interface]) {
        assert!(
            self.num_import_functions.is_none() && self.num_export_functions.is_none(),
            "must call `preprocess_all` exactly once"
        );
        assert!(
            exports.len() <= 1,
            "only one exported interface is currently supported"
        );
        self.num_import_functions =
            Some(u32::try_from(imports.iter().map(|i| i.functions.len()).sum::<usize>()).unwrap());
        self.num_export_functions =
            Some(u32::try_from(exports.iter().map(|i| i.functions.len()).sum::<usize>()).unwrap());
    }

    fn preprocess_one(&mut self, iface: &Interface, _dir: Direction) {
        self.sizes.fill(iface);
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        let _ = (iface, id, name, record, docs);
        todo!()
    }

    fn type_tuple(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        tuple: &Tuple,
        docs: &Docs,
    ) {
        let _ = (iface, id, name, tuple, docs);
        todo!()
    }

    fn type_flags(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        flags: &Flags,
        docs: &Docs,
    ) {
        let _ = (iface, id, name, flags, docs);
        todo!()
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        let _ = (iface, id, name, variant, docs);
        todo!()
    }

    fn type_union(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        union: &Union,
        docs: &Docs,
    ) {
        let _ = (iface, id, name, union, docs);
        todo!()
    }

    fn type_option(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        payload: &Type,
        docs: &Docs,
    ) {
        let _ = (iface, id, name, payload, docs);
        todo!()
    }

    fn type_expected(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        expected: &Expected,
        docs: &Docs,
    ) {
        let _ = (iface, id, name, expected, docs);
        todo!()
    }

    fn type_enum(&mut self, iface: &Interface, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        let _ = (iface, id, name, enum_, docs);
        todo!()
    }

    fn type_resource(&mut self, iface: &Interface, ty: ResourceId) {
        let _ = (iface, ty);
        todo!()
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let _ = (iface, id, name, ty, docs);
        todo!()
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let _ = (iface, id, name, ty, docs);
        todo!()
    }

    fn type_builtin(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let _ = (iface, id, name, name, ty, docs);
        todo!()
    }

    fn import(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");

        // Add the raw Wasm import.
        let wasm_sig = iface.wasm_signature(AbiVariant::GuestImport, func);
        let type_index = self.intern_type(wasm_sig.clone());
        let import_fn_index = self.wit_import(self.imports.len());
        self.imports.import(
            &iface.name,
            Some(&func.name),
            wasm_encoder::EntityType::Function(type_index),
        );

        let existing = self
            .import_fn_name_to_index
            .entry(iface.name.clone())
            .or_default()
            .insert(
                func.name.clone(),
                (import_fn_index, u32::try_from(func.params.len()).unwrap()),
            );
        assert!(existing.is_none());

        self.function_names
            .push((import_fn_index, format!("{}.{}", iface.name, func.name)));

        let mut bindgen = Bindgen::new(self, &wasm_sig, func, abi::LiftLower::LowerArgsLiftResults);
        iface.call(
            AbiVariant::GuestImport,
            abi::LiftLower::LowerArgsLiftResults,
            func,
            &mut bindgen,
        );
        let func_encoder = bindgen.finish();
        self.import_glue_fns.push(func_encoder);
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");

        let wasm_sig = iface.wasm_signature(AbiVariant::GuestExport, func);
        let type_index = self.intern_type(wasm_sig.clone());
        let export_fn_index = self.wit_export(self.exports.len());
        self.exports
            .export(&func.name, wasm_encoder::Export::Function(export_fn_index));
        self.function_names
            .push((export_fn_index, format!("{}.{}", iface.name, func.name)));

        let mut bindgen = Bindgen::new(self, &wasm_sig, func, abi::LiftLower::LiftArgsLowerResults);
        iface.call(
            AbiVariant::GuestExport,
            abi::LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
        );
        let func_encoder = bindgen.finish();
        self.export_glue_fns.push((func_encoder, type_index));
    }

    fn finish_one(&mut self, _iface: &Interface, _files: &mut Files) {
        // Nothing to do until wil finish all interfaces and generate our Wasm
        // glue code.
    }

    fn finish_all(&mut self, files: &mut Files) {
        let mut module = wasm_encoder::Module::new();
        let mut modules = wasm_encoder::ModuleSection::new();
        let mut instances = wasm_encoder::InstanceSection::new();
        let mut aliases = wasm_encoder::AliasSection::new();
        let mut mems = wasm_encoder::MemorySection::new();
        let mut funcs = wasm_encoder::FunctionSection::new();
        let mut globals = wasm_encoder::GlobalSection::new();
        let mut elems = wasm_encoder::ElementSection::new();
        let mut code = wasm_encoder::CodeSection::new();

        self.link_spidermonkey_wasm(&mut modules, &mut instances, &mut aliases);

        // Define the return pointer global.
        globals.global(
            wasm_encoder::GlobalType {
                val_type: wasm_encoder::ValType::I32,
                mutable: true,
            },
            &Instruction::I32Const(0),
        );

        // Re-export `spidermonkey.wasm`'s memory and canonical ABI functions.
        self.exports
            .export("memory", wasm_encoder::Export::Memory(SM_MEMORY));
        self.exports.export(
            "canonical_abi_free",
            wasm_encoder::Export::Function(self.spidermonkey_import("canonical_abi_free")),
        );
        self.exports.export(
            "canonical_abi_realloc",
            wasm_encoder::Export::Function(self.spidermonkey_import("canonical_abi_realloc")),
        );

        // Add the WIT function imports (add their import glue functions) to
        // the module.
        //
        // Each of these functions has the Wasm equivalent of this function
        // signature:
        //
        //     using JSNative = bool (JSContext* cx, unsigned argc, JSValue *vp);
        let js_native_type_index = self.intern_type(WasmSignature {
            params: vec![
                // JSContext *cx
                WasmType::I32,
                // unsigned argc
                WasmType::I32,
                // JSValue *vp
                WasmType::I32,
            ],
            results: vec![
                // bool
                WasmType::I32,
            ],
            retptr: false,
            indirect_params: false,
        });
        for f in &self.import_glue_fns {
            funcs.function(js_native_type_index);
            code.function(f);
        }
        for (f, ty_idx) in &self.export_glue_fns {
            funcs.function(*ty_idx);
            code.function(f);
        }

        // We will use `ref.func` to get a reference to each of our synthesized
        // import glue functions, so we need to declare them as reference-able.
        let func_indices: Vec<u32> = self.wit_import_glue_fn_range().collect();
        if !func_indices.is_empty() {
            elems.declared(
                wasm_encoder::ValType::FuncRef,
                wasm_encoder::Elements::Functions(&func_indices),
            );
        }

        let js_name = self.js_name.display().to_string();
        let js_name_offset = self.data_segments.add(js_name.as_bytes().iter().copied());
        let js_offset = self.data_segments.add(self.js.as_bytes().iter().copied());

        self.define_wizer_initialize(
            &mut funcs,
            &mut code,
            js_name_offset,
            u32::try_from(js_name.len()).unwrap(),
            js_offset,
            u32::try_from(self.js.len()).unwrap(),
        );

        module.section(&self.types).section(&self.imports);

        if !self.import_spidermonkey {
            module.section(&modules).section(&instances);
        }

        mems.memory(self.data_segments.memory_type());
        let data = self.data_segments.take_data();

        // Fill out the `names` section to assist in debugging the generated
        // wasm.
        let mut names = wasm_encoder::NameSection::new();
        self.function_names.sort_by_key(|a| a.0);
        let mut function_names = wasm_encoder::NameMap::new();
        for (i, name) in self.function_names.iter() {
            function_names.append(*i, name);
        }
        names.functions(&function_names);

        self.local_names.sort_by_key(|a| a.0);
        let mut local_names = wasm_encoder::IndirectNameMap::new();
        for (i, names) in self.local_names.iter() {
            local_names.append(*i, names);
        }
        names.locals(&local_names);

        let mut table_names = wasm_encoder::NameMap::new();
        table_names.append(0, "sm_function_table");
        names.tables(&table_names);

        let mut memory_names = wasm_encoder::NameMap::new();
        memory_names.append(SM_MEMORY, "sm_mem");
        memory_names.append(GLUE_MEMORY, "glue_mem");
        names.memories(&memory_names);

        module
            .section(&aliases)
            .section(&funcs)
            .section(&mems)
            .section(&globals)
            .section(&self.exports)
            .section(&elems)
            .section(&code)
            .section(&data)
            .section(&names);

        let wasm = module.finish();

        let js_file_stem = self.js_name.file_stem().unwrap_or_else(|| {
            panic!(
                "input JavaScript file path does not have a file stem: {}",
                self.js_name.display()
            )
        });
        let js_file_stem = js_file_stem.to_str().unwrap_or_else(|| {
            panic!(
                "input JavaScript file path is not UTF-8 representable: {}",
                self.js_name.display()
            )
        });
        let wasm_name = format!("{}.wasm", js_file_stem);

        files.push(&wasm_name, &wasm);
    }
}

trait InstructionSink<'a> {
    fn instruction(&mut self, inst: wasm_encoder::Instruction<'a>);
}

impl<'a> InstructionSink<'a> for wasm_encoder::Function {
    fn instruction(&mut self, inst: wasm_encoder::Instruction<'a>) {
        wasm_encoder::Function::instruction(self, &inst);
    }
}

impl<'a> InstructionSink<'a> for Vec<wasm_encoder::Instruction<'a>> {
    fn instruction(&mut self, inst: wasm_encoder::Instruction<'a>) {
        self.push(inst);
    }
}

const RET_PTR_GLOBAL: u32 = 0;

const SM_MEMORY: u32 = 0;
const GLUE_MEMORY: u32 = 1;

fn convert_ty(ty: WasmType) -> wasm_encoder::ValType {
    match ty {
        WasmType::I32 => wasm_encoder::ValType::I32,
        WasmType::I64 => wasm_encoder::ValType::I64,
        WasmType::F32 => wasm_encoder::ValType::F32,
        WasmType::F64 => wasm_encoder::ValType::F64,
    }
}

struct Bindgen<'a, 'b> {
    gen: &'a mut SpiderMonkeyWasm<'b>,
    sig: &'a WasmSignature,
    lift_lower: abi::LiftLower,
    locals: Vec<wasm_encoder::ValType>,
    js_count: u32,

    blocks: Vec<Vec<Instruction<'a>>>,
    block_results: Vec<Vec<Operand>>,

    /// The `i`th JS operand that is our current iteration element, if any.
    iter_elem: Vec<u32>,

    /// The Wasm local for our current iteration's base pointer, if any.
    iter_base_pointer: Vec<u32>,

    /// Allocations to free after the call.
    ///
    /// `(local holding pointer, local holding length, alignment)`
    to_free: Vec<(u32, u32, u32)>,
}

impl<'a, 'b> Bindgen<'a, 'b> {
    fn new(
        gen: &'a mut SpiderMonkeyWasm<'b>,
        sig: &'a WasmSignature,
        func: &'a Function,
        lift_lower: abi::LiftLower,
    ) -> Self {
        let js_count = match lift_lower {
            abi::LiftLower::LiftArgsLowerResults => 0,
            abi::LiftLower::LowerArgsLiftResults => u32::try_from(func.params.len()).unwrap(),
        };

        let mut insts = vec![];
        if lift_lower == abi::LiftLower::LowerArgsLiftResults && !func.params.is_empty() {
            // Initialize `bindgen.cpp`'s JS value operands vector with the
            // arguments given to this function
            //
            // []
            insts.push(Instruction::LocalGet(1));
            // [i32]
            insts.push(Instruction::LocalGet(2));
            // [i32 i32]
            insts.push(Instruction::Call(
                gen.spidermonkey_import("SMW_fill_operands"),
            ));
            // []
        }

        Bindgen {
            gen,
            sig,
            lift_lower,
            locals: vec![],
            js_count,
            blocks: vec![insts],
            block_results: vec![],
            iter_elem: vec![],
            iter_base_pointer: vec![],
            to_free: vec![],
        }
    }

    fn inst(&mut self, inst: Instruction<'a>) {
        self.current_block().push(inst);
    }

    fn current_block(&mut self) -> &mut Vec<Instruction<'a>> {
        self.blocks.last_mut().unwrap()
    }

    fn pop_block(&mut self) -> (Vec<Instruction<'a>>, Vec<Operand>) {
        (
            self.blocks.pop().unwrap(),
            self.block_results.pop().unwrap(),
        )
    }

    /// Create a new Wasm local for this function and return its index.
    fn new_local(&mut self, ty: wasm_encoder::ValType) -> u32 {
        let offset = match self.lift_lower {
            abi::LiftLower::LiftArgsLowerResults => self.sig.params.len(),
            // `JSNative` functions take three `i32` arguments: cx, argc, and
            // vp.
            abi::LiftLower::LowerArgsLiftResults => 3,
        };
        let idx = u32::try_from(self.locals.len() + offset).unwrap();
        self.locals.push(ty);
        idx
    }

    /// Get the next JS value operand.
    fn next_js(&mut self) -> Operand {
        let js = self.js_count;
        self.js_count += 1;
        Operand::Js(js)
    }

    /// Finish generating these bindings and return the encoded Wasm function.
    fn finish(self) -> wasm_encoder::Function {
        // TODO: Coalesce contiguous locals of the same type here into the
        // compact encoding, like `[(i32, 3)]` rather than `[(i32, 1), (i32, 1),
        // (i32, 1)]`.
        let mut f = wasm_encoder::Function::new(self.locals.into_iter().map(|l| (1, l)));

        // By the time we get here, we should have finished all nested blocks.
        assert_eq!(self.blocks.len(), 1);

        for inst in &self.blocks[0] {
            f.instruction(inst);
        }
        f.instruction(&Instruction::End);
        f
    }
}

/// Operands are locals that either hold the value directly or refer to an index
/// in `bindgen.cpp`'s JS operand vector depending on if we're dealing with a JS
/// or Wasm value:
///
/// * When we are _importing_ a function, we are lifting arguments and lowering
///   results, so `operands` always refer to the `n`th local and `results` refer
///   to the `n`th value in `bindgen.cpp`'s JS value vector.
///
/// * When we are _exporting_ a function, we are lowering arguments and lifting
///   results, so `operands` always refer to the `n`th value in `bindgen.cpp`'s
///   JS value vector and `results` always refer to the `n`th local.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operand {
    /// The `n`th JS value in `bindgen.cpp`'s operand vector.
    Js(u32),
    /// The `n`th Wasm local.
    Wasm(u32),
    /// A "unit" void type
    Unit,
}

impl Operand {
    fn unwrap_js(&self) -> u32 {
        match *self {
            Operand::Js(js) => js,
            Operand::Wasm(_) => panic!("Operand::unwrap_js on a Wasm operand"),
            Operand::Unit => panic!("Operand::unwrap_js on a Unit operand"),
        }
    }
    fn unwrap_wasm(&self) -> u32 {
        match *self {
            Operand::Wasm(w) => w,
            Operand::Js(_) => panic!("Operand::unwrap_wasm on a JS operand"),
            Operand::Unit => panic!("Operand::unwrap_wasm on a Unit operand"),
        }
    }
    fn unwrap_unit(&self) {
        match *self {
            Operand::Unit => {}
            Operand::Wasm(_) => panic!("Operand::unwrap_unit on a Wasm operand"),
            Operand::Js(_) => panic!("Operand::unwrap_unit on a JS operand"),
        }
    }
}

fn pop_wasm(operands: &mut Vec<Operand>) -> u32 {
    match operands.pop() {
        Some(op) => op.unwrap_wasm(),
        None => panic!("`pop_wasm` with an empty stack"),
    }
}

fn pop_js(operands: &mut Vec<Operand>) -> u32 {
    match operands.pop() {
        Some(op) => op.unwrap_js(),
        None => panic!("`pop_js` with an empty stack"),
    }
}

fn sm_mem_arg(offset: u32) -> wasm_encoder::MemArg {
    wasm_encoder::MemArg {
        offset: offset as u64,
        align: 0,
        memory_index: SM_MEMORY,
    }
}

impl abi::Bindgen for Bindgen<'_, '_> {
    type Operand = Operand;

    fn emit(
        &mut self,
        _iface: &Interface,
        inst: &abi::Instruction<'_>,
        operands: &mut Vec<Self::Operand>,
        results: &mut Vec<Self::Operand>,
    ) {
        match inst {
            abi::Instruction::GetArg { nth } => {
                let nth = u32::try_from(*nth).unwrap();
                results.push(match self.lift_lower {
                    abi::LiftLower::LiftArgsLowerResults => Operand::Wasm(nth),
                    abi::LiftLower::LowerArgsLiftResults => Operand::Js(nth),
                });
            }
            abi::Instruction::I32Const { val: _ } => todo!(),
            abi::Instruction::Bitcasts { casts: _ } => todo!(),
            abi::Instruction::ConstZero { tys: _ } => todo!(),
            abi::Instruction::I32Load { offset } => {
                let addr = pop_wasm(operands);
                let local = self.new_local(wasm_encoder::ValType::I32);

                // []
                self.inst(Instruction::LocalGet(addr));
                // [i32]
                self.inst(Instruction::I32Load(sm_mem_arg((*offset as u32).into())));
                // [i32]
                self.inst(Instruction::LocalSet(local));
                // []

                results.push(Operand::Wasm(local));
            }
            abi::Instruction::I32Load8U { offset: _ } => todo!(),
            abi::Instruction::I32Load8S { offset: _ } => todo!(),
            abi::Instruction::I32Load16U { offset: _ } => todo!(),
            abi::Instruction::I32Load16S { offset: _ } => todo!(),
            abi::Instruction::I64Load { offset: _ } => todo!(),
            abi::Instruction::F32Load { offset: _ } => todo!(),
            abi::Instruction::F64Load { offset: _ } => todo!(),
            abi::Instruction::I32Store { offset } => {
                let addr = pop_wasm(operands);
                let val = pop_wasm(operands);

                // []
                self.inst(Instruction::LocalGet(addr));
                // [i32]
                self.inst(Instruction::LocalGet(val));
                // [i32 i32]
                self.inst(Instruction::I32Store(sm_mem_arg((*offset as u32).into())));
                // []
            }
            abi::Instruction::I32Store8 { offset: _ } => todo!(),
            abi::Instruction::I32Store16 { offset: _ } => todo!(),
            abi::Instruction::I64Store { offset: _ } => todo!(),
            abi::Instruction::F32Store { offset: _ } => todo!(),
            abi::Instruction::F64Store { offset: _ } => todo!(),
            abi::Instruction::I32FromChar => todo!(),
            abi::Instruction::I64FromU64 => todo!(),
            abi::Instruction::I64FromS64 => todo!(),
            abi::Instruction::I32FromU32 => {
                let js = pop_js(operands);
                let local = self.new_local(wasm_encoder::ValType::I32);

                // []
                self.inst(Instruction::I32Const(js as i32));
                // [i32]
                self.inst(Instruction::Call(
                    self.gen.spidermonkey_import("SMW_i32_from_u32"),
                ));
                // [i32]
                self.inst(Instruction::LocalSet(local));
                // []

                results.push(Operand::Wasm(local));
            }
            abi::Instruction::I32FromS32 => todo!(),
            abi::Instruction::I32FromU16 => todo!(),
            abi::Instruction::I32FromS16 => todo!(),
            abi::Instruction::I32FromU8 => todo!(),
            abi::Instruction::I32FromS8 => todo!(),
            abi::Instruction::F32FromFloat32 => todo!(),
            abi::Instruction::F64FromFloat64 => todo!(),
            abi::Instruction::S8FromI32 => todo!(),
            abi::Instruction::U8FromI32 => todo!(),
            abi::Instruction::S16FromI32 => todo!(),
            abi::Instruction::U16FromI32 => todo!(),
            abi::Instruction::S32FromI32 => todo!(),
            abi::Instruction::U32FromI32 => {
                let local = pop_wasm(operands);
                let result = self.next_js();

                // []
                self.inst(Instruction::LocalGet(local));
                // [i32]
                self.inst(Instruction::I32Const(result.unwrap_js() as i32));
                // [i32 i32]
                self.inst(Instruction::Call(
                    self.gen.spidermonkey_import("SMW_u32_from_i32"),
                ));
                // []

                results.push(result);
            }
            abi::Instruction::S64FromI64 => todo!(),
            abi::Instruction::U64FromI64 => todo!(),
            abi::Instruction::CharFromI32 => todo!(),
            abi::Instruction::Float32FromF32 => todo!(),
            abi::Instruction::Float64FromF64 => todo!(),
            abi::Instruction::I32FromBorrowedHandle { ty: _ } => todo!(),
            abi::Instruction::I32FromOwnedHandle { ty: _ } => todo!(),
            abi::Instruction::HandleOwnedFromI32 { ty: _ } => todo!(),
            abi::Instruction::HandleBorrowedFromI32 { ty: _ } => todo!(),
            abi::Instruction::ListCanonLower { .. } => todo!(),
            abi::Instruction::StringLower { realloc } => {
                let js = pop_js(operands);
                let ptr = self.new_local(wasm_encoder::ValType::I32);
                let len = self.new_local(wasm_encoder::ValType::I32);

                // Make sure our return pointer area can hold at least two
                // `u32`s, since we will use it that way with
                // `SMW_{list,string}_canon_lower`.
                self.gen.return_pointer_area_size = self.gen.return_pointer_area_size.max(1);
                self.gen.return_pointer_area_align = self.gen.return_pointer_area_align.max(4);

                // []
                self.inst(Instruction::GlobalGet(RET_PTR_GLOBAL));
                // [i32]
                self.inst(Instruction::I32Const(js as _));
                // [i32 i32]
                self.inst(Instruction::Call(
                    self.gen.spidermonkey_import("SMW_string_canon_lower"),
                ));
                // []

                // Read the pointer from the return pointer area.
                //
                // []
                self.inst(Instruction::GlobalGet(RET_PTR_GLOBAL));
                // [i32]
                self.inst(Instruction::I32Load(sm_mem_arg(0)));
                // [i32]
                self.inst(Instruction::LocalSet(ptr));
                // []

                // Read the length from the return pointer area.
                //
                // []
                self.inst(Instruction::GlobalGet(RET_PTR_GLOBAL));
                // [i32]
                self.inst(Instruction::I32Load(sm_mem_arg(4)));
                // [i32]
                self.inst(Instruction::LocalSet(len));
                // []

                // If `realloc` is `None`, then we are responsible for freeing
                // this pointer after the call.
                if realloc.is_none() {
                    self.to_free.push((ptr, len, 1));
                }

                results.push(Operand::Wasm(ptr));
                results.push(Operand::Wasm(len));
            }
            abi::Instruction::ListLower { element, realloc } => {
                let iterable = pop_js(operands);
                let (block, block_results) = self.pop_block();
                assert!(block_results.is_empty());
                let iter_elem = self.iter_elem.pop().unwrap();
                let iter_base_pointer = self.iter_base_pointer.pop().unwrap();

                let length = self.new_local(wasm_encoder::ValType::I32);
                let index = self.new_local(wasm_encoder::ValType::I32);
                let ptr = self.new_local(wasm_encoder::ValType::I32);

                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);

                // []
                self.inst(Instruction::I32Const(iterable as i32));
                // [i32]
                self.inst(Instruction::Call(
                    self.gen.spidermonkey_import("SMW_spread_into_array"),
                ));
                // [i32]
                self.inst(Instruction::LocalSet(length));
                // []

                // `malloc` space for the result.
                self.gen
                    .malloc_dynamic_size(self.blocks.last_mut().unwrap(), length, ptr);

                // Create a new block and loop. The block is so we can branch to
                // it to exit out of the loop.
                //
                // Also re-zero the index since the current block itself might
                // be reused multiple times if it is part of a loop body.
                //
                // []
                self.inst(Instruction::I32Const(0));
                // [i32]
                self.inst(Instruction::LocalSet(index));
                // []
                self.inst(Instruction::Block(wasm_encoder::BlockType::Empty));
                // []
                self.inst(Instruction::Loop(wasm_encoder::BlockType::Empty));
                // []

                // Check the loop's exit condition: `index >= length`.
                //
                // []
                self.inst(Instruction::LocalGet(index));
                // [i32]
                self.inst(Instruction::LocalGet(length));
                // [i32 i32]
                self.inst(Instruction::I32GeU);
                // [i32]
                self.inst(Instruction::BrIf(1));
                // []

                // Update the element for this iteration.
                //
                // []
                self.inst(Instruction::I32Const(iterable as i32));
                // [i32]
                self.inst(Instruction::LocalGet(index));
                // [i32 32]
                self.inst(Instruction::I32Const(iter_elem as i32));
                // [i32 i32 i32]
                self.inst(Instruction::Call(
                    self.gen.spidermonkey_import("SMW_get_array_element"),
                ));
                // []

                // Update the base pointer for this iteration.
                //
                // []
                self.inst(Instruction::LocalGet(index));
                // [i32]
                self.inst(Instruction::I32Const(u32::try_from(size).unwrap() as _));
                // [i32 i32]
                self.inst(Instruction::I32Mul);
                // [i32]
                self.inst(Instruction::LocalGet(ptr));
                // [i32 i32]
                self.inst(Instruction::I32Add);
                // [i32]
                self.inst(Instruction::LocalSet(iter_base_pointer));
                // []

                // Now do include the snippet that lowers a single list element!
                self.current_block().extend(block);

                // Increment our index counter.
                //
                // []
                self.inst(Instruction::LocalGet(index));
                // [i32]
                self.inst(Instruction::I32Const(1));
                // [i32 i32]
                self.inst(Instruction::I32Add);
                // [i32]
                self.inst(Instruction::LocalSet(index));
                // []

                // Unconditionally jump back to the loop head, and close out our blocks.
                //
                // []
                self.inst(Instruction::Br(0));
                // []
                self.inst(Instruction::End);
                // []
                self.inst(Instruction::End);
                // []

                // If `realloc` is `None`, then we are responsible for freeing
                // this pointer after the call.
                if realloc.is_none() {
                    self.to_free
                        .push((ptr, length, u32::try_from(align).unwrap()));
                }

                results.push(Operand::Wasm(ptr));
                results.push(Operand::Wasm(length));
            }
            abi::Instruction::ListCanonLift { .. } => todo!(),
            abi::Instruction::StringLift { free } => {
                let len = pop_wasm(operands);
                let ptr = pop_wasm(operands);
                let result = self.next_js();

                // []
                self.inst(Instruction::LocalGet(ptr));
                // [i32]
                self.inst(Instruction::LocalGet(len));
                // [i32 i32]
                self.inst(Instruction::I32Const(result.unwrap_js() as i32));
                // [i32 i32 i32]
                self.inst(Instruction::Call(
                    self.gen.spidermonkey_import("SMW_string_canon_lift"),
                ));
                // []

                if let Some(free) = free {
                    // []
                    self.inst(Instruction::LocalGet(ptr));
                    // [i32]
                    self.inst(Instruction::LocalGet(len));
                    // [i32 i32]
                    self.inst(Instruction::I32Const(1));
                    // [i32 i32 i32]
                    self.inst(Instruction::Call(self.gen.spidermonkey_import(free)));
                    // []
                }

                results.push(result);
            }
            abi::Instruction::ListLift {
                element,
                free,
                ty: _,
            } => {
                let len = pop_wasm(operands);
                let ptr = pop_wasm(operands);
                let (block, block_results) = self.pop_block();
                assert_eq!(block_results.len(), 1);
                let iter_base_pointer = self.iter_base_pointer.pop().unwrap();

                let index = self.new_local(wasm_encoder::ValType::I32);

                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);

                let result = self.next_js();

                // Create a new JS array object that will be the result of this
                // lifting.
                //
                // []
                self.inst(Instruction::I32Const(result.unwrap_js() as i32));
                // [i32]
                self.inst(Instruction::Call(
                    self.gen.spidermonkey_import("SMW_new_array"),
                ));
                // []

                // Create a block and a loop. The block is for branching to when
                // we need to exit the loop.
                //
                // Also re-zero the loop index because it might be reused across
                // multiple loops if the current block itself is also a loop
                // body.
                //
                // []
                self.inst(Instruction::Block(wasm_encoder::BlockType::Empty));
                // []
                self.inst(Instruction::I32Const(0));
                // [i32]
                self.inst(Instruction::LocalSet(index));
                // []
                self.inst(Instruction::Loop(wasm_encoder::BlockType::Empty));
                // []

                // Check for our loop's exit condition: `index >= len`.
                //
                // []
                self.inst(Instruction::LocalGet(index));
                // [i32]
                self.inst(Instruction::LocalGet(len));
                // [i32 i32]
                self.inst(Instruction::I32GeU);
                // [i32]
                self.inst(Instruction::BrIf(1));
                // []

                // Update the base pointer for this iteration.
                //
                // []
                self.inst(Instruction::LocalGet(index));
                // [i32]
                self.inst(Instruction::I32Const(u32::try_from(size).unwrap() as _));
                // [i32 i32]
                self.inst(Instruction::I32Mul);
                // [i32]
                self.inst(Instruction::LocalGet(ptr));
                // [i32 i32]
                self.inst(Instruction::I32Add);
                // [i32]
                self.inst(Instruction::LocalSet(iter_base_pointer));
                // []

                self.current_block().extend(block);

                // Append the result of this iteration's lifting to our JS array.
                //
                // []
                self.inst(Instruction::I32Const(result.unwrap_js() as i32));
                // [i32]
                self.inst(Instruction::I32Const(block_results[0].unwrap_js() as i32));
                // [i32 i32]
                self.inst(Instruction::Call(
                    self.gen.spidermonkey_import("SMW_array_push"),
                ));
                // []

                // Increment the index variable, unconditionally jump back to
                // the head of the loop, and close out our blocks.
                //
                // []
                self.inst(Instruction::I32Const(1));
                // [i32]
                self.inst(Instruction::LocalGet(index));
                // [i32 i32]
                self.inst(Instruction::I32Add);
                // [i32]
                self.inst(Instruction::LocalSet(index));
                // []
                self.inst(Instruction::Br(0));
                // []
                self.inst(Instruction::End);
                // []
                self.inst(Instruction::End);
                // []

                if let Some(free) = free {
                    // []
                    self.inst(Instruction::LocalGet(ptr));
                    // [i32]
                    self.inst(Instruction::LocalGet(len));
                    // [i32 i32]
                    self.inst(Instruction::I32Const(u32::try_from(align).unwrap() as _));
                    // [i32 i32 i32]
                    self.inst(Instruction::Call(self.gen.spidermonkey_import(free)));
                    // []
                }

                results.push(result);
            }
            abi::Instruction::IterElem { element: _ } => {
                let iter_elem = self.next_js();
                self.iter_elem.push(iter_elem.unwrap_js());
                results.push(iter_elem);
            }
            abi::Instruction::IterBasePointer => {
                let iter_base_pointer = self.new_local(wasm_encoder::ValType::I32);
                self.iter_base_pointer.push(iter_base_pointer);
                results.push(Operand::Wasm(iter_base_pointer));
            }

            abi::Instruction::RecordLower {
                record: _,
                name: _,
                ty: _,
            } => todo!(),
            abi::Instruction::RecordLift {
                record: _,
                name: _,
                ty: _,
            } => todo!(),
            abi::Instruction::TupleLower { tuple: _, ty: _ } => todo!(),
            abi::Instruction::TupleLift { tuple: _, ty: _ } => todo!(),
            abi::Instruction::FlagsLower {
                flags: _,
                name: _,
                ty: _,
            } => todo!(),
            abi::Instruction::FlagsLift {
                flags: _,
                name: _,
                ty: _,
            } => todo!(),
            abi::Instruction::VariantPayloadName => todo!(),
            abi::Instruction::VariantLower {
                variant: _,
                name: _,
                ty: _,
                results: _,
            } => todo!(),
            abi::Instruction::VariantLift {
                variant: _,
                name: _,
                ty: _,
            } => todo!(),
            abi::Instruction::OptionLower { .. } => todo!(),
            abi::Instruction::OptionLift { .. } => todo!(),
            abi::Instruction::ExpectedLower { .. } => todo!(),
            abi::Instruction::ExpectedLift { .. } => todo!(),
            abi::Instruction::UnionLower { .. } => todo!(),
            abi::Instruction::UnionLift { .. } => todo!(),
            abi::Instruction::EnumLower {
                enum_: _,
                name: _,
                ty: _,
            } => todo!(),
            abi::Instruction::EnumLift {
                enum_: _,
                name: _,
                ty: _,
            } => todo!(),
            abi::Instruction::CallWasm { iface, name, sig } => {
                // Push the Wasm arguments.
                //
                // []
                let locals: Vec<_> = sig.params.iter().map(|_| pop_wasm(operands)).collect();
                for local in locals.into_iter().rev() {
                    self.inst(Instruction::LocalGet(local));
                }
                // [A...]

                let func_index = self
                    .gen
                    .import_fn_name_to_index
                    .get(&iface.name)
                    .unwrap()
                    .get(*name)
                    .unwrap()
                    .0;
                self.inst(Instruction::Call(func_index));
                // [R...]

                // Allocate locals for the results and pop the return values off
                // the Wasm stack, saving each of them to the associated local.
                let locals: Vec<_> = sig
                    .results
                    .iter()
                    .map(|ty| self.new_local(convert_ty(*ty)))
                    .collect();
                // [R...]
                for l in locals.iter().rev() {
                    self.inst(Instruction::LocalSet(*l));
                }
                // []

                results.extend(locals.into_iter().map(Operand::Wasm));

                for (ptr, len, alignment) in mem::replace(&mut self.to_free, vec![]) {
                    // []
                    self.inst(Instruction::LocalGet(ptr));
                    // [i32]
                    self.inst(Instruction::LocalGet(len));
                    // [i32 i32]
                    self.inst(Instruction::I32Const(alignment as _));
                    // [i32 i32 i32]
                    self.inst(Instruction::Call(
                        self.gen.spidermonkey_import("canonical_abi_free"),
                    ));
                    // []
                }
            }
            abi::Instruction::CallInterface { module: _, func } => {
                // TODO: Rather than always dynamically pushing all of our JS
                // arguments, make `SMW_call_{0,1,...,n}` up to the largest
                // common `n` so we can directly pass the arguments for most
                // function calls.

                // Push the JS arguments.
                let js_args: Vec<_> = func.params.iter().map(|_| pop_js(operands)).collect();
                for js in js_args.into_iter().rev() {
                    // []
                    self.inst(Instruction::I32Const(js as _));
                    // [i32]
                    self.inst(Instruction::Call(
                        self.gen.spidermonkey_import("SMW_push_arg"),
                    ));
                    // []
                }

                // TODO: Rather than `malloc`ing the name for each call, we
                // should pre-`malloc` them in the `wizer.initialize` function,
                // add a global to the glue module that points to the
                // pre-`malloc`ed name for each exported function, and then
                // reuse those that pre-`malloc`ed name on each call.

                // Malloc space for the function name.
                let func_name_local = self.new_local(wasm_encoder::ValType::I32);
                self.gen.malloc_static_size(
                    self.blocks.last_mut().unwrap(),
                    u32::try_from(func.name.len()).unwrap() + 1,
                    func_name_local,
                );

                // Copy the function name from the glue Wasm module's memory
                // into the `malloc`ed space in the `spidermonkey.wasm`'s
                // memory.
                let func_name_offset = self
                    .gen
                    .data_segments
                    .add(func.name.to_snake_case().as_bytes().iter().copied());
                self.gen.copy_to_smw(
                    self.blocks.last_mut().unwrap(),
                    func_name_offset,
                    func_name_local,
                    u32::try_from(func.name.len()).unwrap(),
                );

                let (first_result, num_results) = match &func.result {
                    // If there aren't any function results, then this argument
                    // to `SMW_call` is going to be ignored. Use a highly
                    // visible placeholder so that if this is ever accidentally
                    // used it is easier to debug.
                    Type::Unit => {
                        results.push(Operand::Unit);
                        (0xffffffff, 0)
                    }
                    _ => {
                        let js = self.next_js();
                        results.push(js);
                        (js.unwrap_js(), 1)
                    }
                };

                // Make the call.
                //
                // []
                self.inst(Instruction::LocalGet(func_name_local));
                // [i32]
                self.inst(Instruction::I32Const(
                    i32::try_from(func.name.len()).unwrap(),
                ));
                // [i32 i32]
                self.inst(Instruction::I32Const(num_results));
                // [i32 i32 i32]
                self.inst(Instruction::I32Const(first_result as i32));
                // [i32 i32 i32 i32]
                self.inst(Instruction::Call(self.gen.spidermonkey_import("SMW_call")));
                // []
            }

            abi::Instruction::CallWasmAsyncExport { .. } => todo!(),
            abi::Instruction::CallWasmAsyncImport { .. } => todo!(),

            abi::Instruction::Return { func, amt } => {
                match self.lift_lower {
                    abi::LiftLower::LowerArgsLiftResults => {
                        match &func.result {
                            Type::Unit => {
                                operands[0].unwrap_unit();
                            }
                            _ => {
                                // Attach the return values to the `JS::CallArgs`:
                                // build up the return values via a series of
                                // `SMW_push_return_value` calls, followed by a
                                // single `SMW_finish_returns` call.
                                //
                                // TODO: introduce fast path intrinsics for common
                                // small numbers of return values so that we don't
                                // have to do multiple intrinsic calls here, and can
                                // instead do a single `SMW_return_{1,2,...,n}`
                                // call.
                                let val = pop_js(operands);
                                // []
                                self.inst(Instruction::I32Const(val as _));
                                // [i32]
                                self.inst(Instruction::Call(
                                    self.gen.spidermonkey_import("SMW_push_return_value"),
                                ));
                                // []
                                self.inst(Instruction::LocalGet(1));
                                // [i32]
                                self.inst(Instruction::LocalGet(2));
                                // [i32 i32]
                                self.inst(Instruction::Call(
                                    self.gen.spidermonkey_import("SMW_finish_returns"),
                                ));
                                // []
                            }
                        }

                        // NB: only clear the JS operands after we've attached
                        // the return value(s) to the `JS::CallArgs`.
                        self.gen.clear_js_operands(self.blocks.last_mut().unwrap());

                        // Return `true`, meaning that a JS exception was not thrown.
                        //
                        // []
                        self.inst(Instruction::I32Const(1));
                        // [i32]
                        self.inst(Instruction::Return);
                        // []
                    }
                    abi::LiftLower::LiftArgsLowerResults => {
                        self.gen.clear_js_operands(self.blocks.last_mut().unwrap());

                        // Get the return values out of their locals and push
                        // them onto the Wasm stack.
                        //
                        // []
                        for _ in 0..*amt {
                            let local = pop_wasm(operands);
                            self.inst(Instruction::LocalGet(local));
                        }
                        // [R...]
                        self.inst(Instruction::Return);
                        // []
                    }
                }
            }

            abi::Instruction::UnitLower { .. } => {
                operands[0].unwrap_unit();
            }
            abi::Instruction::UnitLift { .. } => {
                results.push(Operand::Unit);
            }

            abi::Instruction::I32FromBool { .. } => todo!(),
            abi::Instruction::BoolFromI32 { .. } => todo!(),

            abi::Instruction::ReturnAsyncExport { .. } => todo!(),
            abi::Instruction::ReturnAsyncImport { .. } => todo!(),

            abi::Instruction::Malloc { .. } => todo!(),
            abi::Instruction::Free { .. } => todo!(),
        }
    }

    fn return_pointer(&mut self, _iface: &Interface, size: usize, align: usize) -> Self::Operand {
        self.gen.return_pointer_area_size = self.gen.return_pointer_area_size.max(size);
        self.gen.return_pointer_area_align = self.gen.return_pointer_area_align.max(align);
        let local = self.new_local(wasm_encoder::ValType::I32);

        // []
        self.inst(Instruction::GlobalGet(RET_PTR_GLOBAL));
        // [i32]
        self.inst(Instruction::LocalSet(local));
        // []

        Operand::Wasm(local)
    }

    fn push_block(&mut self) {
        self.blocks.push(vec![]);
    }

    fn finish_block(&mut self, results: &mut Vec<Self::Operand>) {
        self.block_results.push(results.to_vec());
    }

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
    }

    fn is_list_canonical(&self, _iface: &Interface, _ty: &Type) -> bool {
        // TODO: we will want to support canonical lists for the various typed
        // arrays.
        false
    }
}
