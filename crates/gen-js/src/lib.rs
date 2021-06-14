#![allow(warnings)]

use heck::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use witx_bindgen_gen_core::witx2::abi::{
    Abi, Bindgen, CallMode, Instruction, WasmType, WitxInstruction,
};
use witx_bindgen_gen_core::{witx2::*, Files, Generator, TypeInfo, Types};

#[derive(Default)]
pub struct Js {
    tmp: usize,
    src: Source,
    opts: Opts,
    imports: HashMap<String, Vec<Import>>,
    block_storage: Vec<String>,
    blocks: Vec<String>,
    in_import: bool,
    sizes: SizeAlign,
    needs_clamp_guest: bool,
    needs_clamp_host: bool,
    needs_clamp_host64: bool,
    needs_get_export: bool,
    needs_data_view: bool,
    needs_validate_f32: bool,
    needs_validate_f64: bool,
    needs_validate_guest_char: bool,
    needs_validate_host_char: bool,
}

struct Import {
    name: String,
    src: Source,
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    // ...
}

impl Opts {
    pub fn build(self) -> Js {
        let mut r = Js::new();
        r.opts = self;
        r
    }
}

impl Js {
    pub fn new() -> Js {
        Js::default()
    }

    fn call_mode(&self) -> CallMode {
        if self.in_import {
            CallMode::NativeImport
        } else {
            CallMode::WasmExport
        }
    }

    fn tmp(&mut self) -> usize {
        let ret = self.tmp;
        self.tmp += 1;
        ret
    }

    fn print_intrinsics(&mut self) {
        if self.needs_clamp_guest {
            self.src.js("function clamp_guest(i, min, max) {
                if (i < min || i > max) \
                    throw new RangeError(`must be between ${min} and ${max}`);
                return i;
            }\n");
        }

        if self.needs_clamp_host {
            self.src.js("function clamp_host(i, min, max) {
                if (!Number.isInteger(i)) \
                    throw new TypeError(`must be an integer`);
                if (i < min || i > max) \
                    throw new RangeError(`must be between ${min} and ${max}`);
                return i;
            }\n");
        }
        if self.needs_clamp_host64 {
            self.src.js("function clamp_host64(i, min, max) {
                if (typeof i !== 'bigint') \
                    throw new TypeError(`must be a bigint`);
                if (i < min || i > max) \
                    throw new RangeError(`must be between ${min} and ${max}`);
                return i;
            }\n");
        }
        if self.needs_data_view {
            self.src.js("let DATA_VIEW = new DataView();\n");
            // TODO: hardcoded `memory`
            self.src.js("function data_view() {
                const mem = get_export(\"memory\");
                if (DATA_VIEW.buffer !== mem.buffer) \
                    DATA_VIEW = new DataView(mem.buffer);
                return DATA_VIEW;
            }\n");
        }

        if self.needs_validate_f32 {
            // TODO: test removing the isNan test and make sure something fails
            self.src.js("function validate_f32(val) {
                if (typeof val !== 'number') \
                    throw new TypeError(`must be a number`);
                if (!Number.isNan(val) && Math.fround(val) !== val) \
                    throw new RangeError(`must be representable as f32`);
                return val;
            }\n");
        }

        if self.needs_validate_f64 {
            self.src.js("function validate_f64(val) {
                if (typeof val !== 'number') \
                    throw new TypeError(`must be a number`);
                return val;
            }\n");
        }

        if self.needs_validate_guest_char {
            self.src.js("function validate_guest_char(i) {
                if ((i > 0x10ffff) || (i >= 0xd800 && i <= 0xdfff)) \
                    throw new RangeError(`not a valid char`);
                return String.fromCodePoint(i);
            }\n");
        }

        if self.needs_validate_host_char {
            // TODO: this is incorrect. It at least allows strings of length > 0
            // but it probably doesn't do the right thing for unicode or invalid
            // utf16 strings either.
            self.src.js("function validate_host_char(s) {
                if (typeof s !== 'string') \
                    throw new TypeError(`must be a string`);
                return s.codePointAt(0);
            }\n");
        }
    }

    fn clamp_guest<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        self.needs_clamp_guest = true;
        results.push(format!("clamp_guest({}, {}, {})", operands[0], min, max));
    }

    fn clamp_host<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        self.needs_clamp_host = true;
        results.push(format!("clamp_host({}, {}, {})", operands[0], min, max));
    }

    fn clamp_host64<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        self.needs_clamp_host64 = true;
        results.push(format!("clamp_host64({}, {}n, {}n)", operands[0], min, max));
    }

    fn need_data_view(&mut self) {
        self.needs_get_export = true;
        self.needs_data_view = true;
    }

    fn store(&mut self, method: &str, offset: i32, operands: &[String]) {
        self.need_data_view();
        self.src.js(&format!(
            "data_view().{}({} + {}, {}, true);\n",
            method, operands[1], offset, operands[0]
        ));
    }
}

impl Generator for Js {
    fn preprocess(&mut self, iface: &Interface, import: bool) {
        let mode = self.call_mode();
        self.sizes.fill(mode, iface);
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        // TODO: should do something here probably
        drop((iface, id, name, record, docs));
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        panic!()
    }

    fn type_resource(&mut self, iface: &Interface, ty: ResourceId) {
        panic!()
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        panic!()
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        panic!()
    }

    fn type_pointer(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        const_: bool,
        ty: &Type,
        docs: &Docs,
    ) {
        panic!()
    }

    fn type_builtin(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        panic!()
    }

    fn type_push_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
        panic!()
    }

    fn type_pull_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
        panic!()
    }

    fn import(&mut self, iface: &Interface, func: &Function) {
        self.in_import = true;
        self.tmp = 0;
        let prev = mem::take(&mut self.src);
        let sig = iface.wasm_signature(self.call_mode(), func);
        let args = (0..sig.params.len())
            .map(|i| format!("arg{}", i))
            .collect::<Vec<_>>()
            .join(", ");
        self.src.js(&format!("function({}) {{\n", args));
        iface.call(self.call_mode(), func, self);
        self.src.js("}");

        let src = mem::replace(&mut self.src, prev);
        self.imports
            .entry(iface.name.to_string())
            .or_insert(Vec::new())
            .push(Import {
                name: func.name.to_string(),
                src,
            });
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        panic!()
    }

    fn finish(&mut self, files: &mut Files) {
        for (module, funcs) in mem::take(&mut self.imports) {
            let module = module.to_snake_case();
            let get_export = if self.needs_get_export {
                ", get_export"
            } else {
                ""
            };
            self.src.js(&format!(
                "export function add_{}_to_imports(imports, obj{}) {{\n",
                module, get_export,
            ));
            self.print_intrinsics();
            self.src.js(&format!(
                "if (!(\"{0}\" in imports)) imports[\"{0}\"] = {{}};\n",
                module,
            ));

            for f in funcs {
                let func = f.name.to_snake_case();
                self.src.js(&format!(
                    "imports[\"{}\"][\"{}\"] = {};\n",
                    module,
                    func,
                    f.src.js.trim(),
                ));
            }
            self.src.js("}");
        }

        files.push("bindings.js", self.src.js.as_bytes());
        files.push("bindings.d.ts", self.src.ts.as_bytes());
    }
}

impl Bindgen for Js {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.sizes
    }

    fn push_block(&mut self) {
        panic!()
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        panic!()
    }

    fn allocate_typed_space(&mut self, _iface: &Interface, _ty: TypeId) -> String {
        unimplemented!()
    }

    fn i64_return_pointer_area(&mut self, _amt: usize) -> String {
        unimplemented!()
    }

    fn emit(
        &mut self,
        iface: &Interface,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        //let mut top_as = |cvt: &str| {
        //    let mut s = operands.pop().unwrap();
        //    s.push_str(" as ");
        //    s.push_str(cvt);
        //    results.push(s);
        //};

        //let mut try_from = |cvt: &str, operands: &[String], results: &mut Vec<String>| {
        //    self.needs_bad_int = true;
        //    let result = format!("{}::try_from({}).map_err(bad_int)?", cvt, operands[0]);
        //    results.push(result);
        //};

        match inst {
            Instruction::GetArg { nth } => results.push(format!("arg{}", nth)),
            //    Instruction::I32Const { val } => results.push(format!("{}i32", val)),
            //    Instruction::ConstZero { tys } => {
            //        for ty in tys.iter() {
            //            match ty {
            //                WasmType::I32 => results.push("0i32".to_string()),
            //                WasmType::I64 => results.push("0i64".to_string()),
            //                WasmType::F32 => results.push("0.0f32".to_string()),
            //                WasmType::F64 => results.push("0.0f64".to_string()),
            //            }
            //        }
            //    }

            // The representation of i32 in JS is a number, so 8/16-bit values
            // get further clamped to ensure that the upper bits aren't set when
            // we pass the value, ensuring that only the right number of bits
            // are transferred.
            Instruction::U8FromI32 => self.clamp_guest(results, operands, u8::MIN, u8::MAX),
            Instruction::S8FromI32 => self.clamp_guest(results, operands, i8::MIN, i8::MAX),
            Instruction::U16FromI32 => self.clamp_guest(results, operands, u16::MIN, u16::MAX),
            Instruction::S16FromI32 => self.clamp_guest(results, operands, i16::MIN, i16::MAX),
            // Use `>>>0` to ensure the bits of the number are treated as
            // unsigned.
            Instruction::U32FromI32 => results.push(format!("{} >>> 0", operands[0])),
            // All bigints coming from wasm are treated as signed, so convert
            // it to ensure it's treated as unsigned.
            Instruction::U64FromI64 => results.push(format!("BigInt.asUintN(64, {})", operands[0])),
            // Nothing to do signed->signed where the representations are the
            // same.
            Instruction::S32FromI32 | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap())
            }

            // All values coming from the host and going to wasm need to have
            // their ranges validated, since the host could give us any value.
            Instruction::I32FromU8 => self.clamp_host(results, operands, u8::MIN, u8::MAX),
            Instruction::I32FromS8 => self.clamp_host(results, operands, i8::MIN, i8::MAX),
            Instruction::I32FromU16 => self.clamp_host(results, operands, u16::MIN, u16::MAX),
            Instruction::I32FromS16 => self.clamp_host(results, operands, i16::MIN, i16::MAX),
            Instruction::I32FromU32 => self.clamp_host(results, operands, u32::MIN, u32::MAX),
            Instruction::I32FromS32 => self.clamp_host(results, operands, i32::MIN, i32::MAX),
            Instruction::I64FromU64 => self.clamp_host64(results, operands, u64::MIN, u64::MAX),
            Instruction::I64FromS64 => self.clamp_host64(results, operands, i64::MIN, i64::MAX),

            // The native representation in JS of f32 and f64 is just a number,
            // so there's nothing to do here. Everything wasm gives us is
            // representable in JS.
            Instruction::If32FromF32 | Instruction::If64FromF64 => {
                results.push(operands.pop().unwrap())
            }

            // For f32 coming from the host we need to validate that the value
            // is indeed a number and that the 32-bit value matches the
            // original value.
            Instruction::F32FromIf32 => {
                self.needs_validate_f32 = true;
                results.push(format!("validate_f32({})", operands[0]));
            }

            // Similar to f32, but no range checks, just checks it's a number
            Instruction::F64FromIf64 => {
                self.needs_validate_f64 = true;
                results.push(format!("validate_f64({})", operands[0]));
            }

            // Validate that i32 values coming from wasm are indeed valid code
            // points.
            Instruction::CharFromI32 => {
                self.needs_validate_guest_char = true;
                results.push(format!("validate_guest_char({})", operands[0]));
            }

            // Validate that strings are indeed 1 character long and valid
            // unicode.
            Instruction::I32FromChar => {
                self.needs_validate_host_char = true;
                results.push(format!("validate_host_char({})", operands[0]));
            }

            //    Instruction::Bitcasts { casts } => {
            //        witx_bindgen_gen_rust::bitcast(casts, operands, results)
            //    }

            //    Instruction::I32FromOwnedHandle { ty } => {
            //        let name = &iface.resources[*ty].name;
            //        results.push(format!(
            //            "_tables.{}_table.insert({}) as i32",
            //            name.to_snake_case(),
            //            operands[0]
            //        ));
            //    }
            //    Instruction::HandleBorrowedFromI32 { ty } => {
            //        let name = &iface.resources[*ty].name;
            //        if self.is_dtor {
            //            results.push(format!(
            //                "_tables.{}_table.remove(({}) as u32).map_err(|e| {{
            //                    wasmtime::Trap::new(format!(\"failed to remove handle: {{}}\", e))
            //                }})?",
            //                name.to_snake_case(),
            //                operands[0]
            //            ));
            //        } else {
            //            results.push(format!(
            //                "_tables.{}_table.get(({}) as u32).ok_or_else(|| {{
            //                    wasmtime::Trap::new(\"invalid handle index\")
            //                }})?",
            //                name.to_snake_case(),
            //                operands[0]
            //            ));
            //        }
            //    }
            //    Instruction::I32FromBorrowedHandle { .. } => {
            //        results.push(format!("{}.0", operands[0]));
            //    }
            //    Instruction::HandleOwnedFromI32 { ty } => {
            //        let name = &iface.resources[*ty].name;
            //        results.push(format!(
            //            "{}({}, std::mem::ManuallyDrop::new(self.{}_close.clone()))",
            //            name.to_camel_case(),
            //            operands[0],
            //            name.to_snake_case(),
            //        ));
            //    }
            Instruction::RecordLower { ty, record, .. } => {
                if record.is_tuple() {
                    let tmp = self.tmp();
                    let mut expr = "const [".to_string();
                    for i in 0..record.fields.len() {
                        if i > 0 {
                            expr.push_str(", ");
                        }
                        let name = format!("tuple{}_{}", tmp, i);
                        expr.push_str(&name);
                        results.push(name);
                    }
                    self.src.js(&format!("{}] = {};\n", expr, operands[0]));
                } else {
                    let tmp = self.tmp();
                    let mut expr = "const {".to_string();
                    for (i, field) in record.fields.iter().enumerate() {
                        if i > 0 {
                            expr.push_str(", ");
                        }
                        let name = format!("v{}_{}", tmp, i);
                        expr.push_str(&field.name.to_camel_case());
                        expr.push_str(": ");
                        expr.push_str(&name);
                        results.push(name);
                    }
                    self.src.js(&format!("{}}} = {};\n", expr, operands[0]));
                }
            }
            Instruction::RecordLift { ty, record, .. } => {
                if record.is_tuple() {
                    results.push(format!("[{}]", operands.join(", ")));
                } else {
                    let mut result = "{\n".to_string();
                    for (field, op) in record.fields.iter().zip(operands) {
                        result.push_str(&format!("{}: {},\n", field.name.to_camel_case(), op));
                    }
                    result.push_str("}");
                    results.push(result);
                }
            }

            //    Instruction::FlagsLower { record, .. } => {
            //        let tmp = self.tmp();
            //        self.push_str(&format!("let flags{} = {};\n", tmp, operands[0]));
            //        for i in 0..record.num_i32s() {
            //            results.push(format!("(flags{}.bits >> {}) as i32", tmp, i * 32));
            //        }
            //    }
            //    Instruction::FlagsLower64 { .. } => {
            //        results.push(format!("({}).bits as i64", operands[0]));
            //    }
            //    Instruction::FlagsLift { record, name, .. }
            //    | Instruction::FlagsLift64 { record, name, .. } => {
            //        self.needs_validate_flags = true;
            //        let repr = iface
            //            .flags_repr(record)
            //            .expect("unsupported number of flags");
            //        let mut flags = String::from("0");
            //        for (i, op) in operands.iter().enumerate() {
            //            flags.push_str(&format!("| (i64::from({}) << {})", op, i * 32));
            //        }
            //        results.push(format!(
            //            "validate_flags(
            //                {},
            //                {name}::all().bits() as i64,
            //                \"{name}\",
            //                |b| {name} {{ bits: b as {ty} }}
            //            )?",
            //            flags,
            //            name = name.to_camel_case(),
            //            ty = int_repr(repr),
            //        ));
            //    }

            //    Instruction::VariantPayload => results.push("e".to_string()),

            //    Instruction::VariantLower {
            //        variant,
            //        nresults,
            //        ty,
            //        ..
            //    } => {
            //        let blocks = self
            //            .blocks
            //            .drain(self.blocks.len() - variant.cases.len()..)
            //            .collect::<Vec<_>>();
            //        self.variant_lower(
            //            iface,
            //            *ty,
            //            variant,
            //            *nresults,
            //            &operands[0],
            //            results,
            //            blocks,
            //        );
            //    }

            //    Instruction::VariantLift { variant, name, ty } => {
            //        let blocks = self
            //            .blocks
            //            .drain(self.blocks.len() - variant.cases.len()..)
            //            .collect::<Vec<_>>();
            //        let mut result = format!("match ");
            //        result.push_str(&operands[0]);
            //        result.push_str(" {\n");
            //        for (i, (case, block)) in variant.cases.iter().zip(blocks).enumerate() {
            //            result.push_str(&i.to_string());
            //            result.push_str(" => ");
            //            self.variant_lift_case(iface, *ty, variant, case, &block, &mut result);
            //            result.push_str(",\n");
            //        }
            //        let variant_name = name.map(|s| s.to_camel_case());
            //        let variant_name = variant_name.as_deref().unwrap_or_else(|| {
            //            if variant.is_bool() {
            //                "bool"
            //            } else if variant.as_expected().is_some() {
            //                "Result"
            //            } else if variant.as_option().is_some() {
            //                "Option"
            //            } else {
            //                unimplemented!()
            //            }
            //        });
            //        result.push_str("_ => return Err(invalid_variant(\"");
            //        result.push_str(&variant_name);
            //        result.push_str("\")),\n");
            //        result.push_str("}");
            //        results.push(result);
            //        self.needs_invalid_variant = true;
            //    }

            //    Instruction::ListCanonLower { element, realloc } => {
            //        // Lowering only happens when we're passing lists into wasm,
            //        // which forces us to always allocate, so this should always be
            //        // `Some`.
            //        //
            //        // Note that the size of a list of `char` is 1 because it's
            //        // encoded as utf-8, otherwise it's just normal contiguous array
            //        // elements.
            //        let realloc = realloc.unwrap();
            //        self.needs_functions
            //            .insert(realloc.to_string(), NeededFunction::Realloc);
            //        let (size, align) = match element {
            //            Type::Char => (1, 1),
            //            _ => (self.sizes.size(element), self.sizes.align(element)),
            //        };

            //        // Store the operand into a temporary...
            //        let tmp = self.tmp();
            //        let val = format!("vec{}", tmp);
            //        self.push_str(&format!("let {} = {};\n", val, operands[0]));

            //        // ... and then realloc space for the result in the guest module
            //        let ptr = format!("ptr{}", tmp);
            //        self.push_str(&format!(
            //            "let {} = func_{}.call(&mut caller, (0, 0, ({}.len() as i32) * {}, {}))?;\n",
            //            ptr, realloc, val, size, align
            //        ));
            //        self.caller_memory_available = false; // invalidated from above

            //        // ... and then copy over the result.
            //        //
            //        // Note the unsafety here, in general it's not safe to copy
            //        // from arbitrary types on the host as a slice of bytes, but in
            //        // this case we should be able to get away with it since
            //        // canonical lowerings have the same memory representation on
            //        // the host as in the guest.
            //        let mem = self.memory_src();
            //        self.push_str(&format!(
            //            "{}.store({}, unsafe {{ slice_as_bytes({}.as_ref()) }})?;\n",
            //            mem, ptr, val
            //        ));
            //        self.needs_store = true;
            //        self.needs_memory = true;
            //        self.needs_slice_as_bytes = true;
            //        results.push(ptr);
            //        results.push(format!("{}.len() as i32", val));
            //    }

            //    Instruction::ListCanonLift { element, free } => {
            //        // Note the unsafety here. This is possibly an unsafe operation
            //        // because the representation of the target must match the
            //        // representation on the host, but `ListCanonLift` is only
            //        // generated for types where that's true, so this should be
            //        // safe.
            //        match free {
            //            Some(free) => {
            //                self.needs_memory = true;
            //                self.needs_copy_slice = true;
            //                self.needs_functions
            //                    .insert(free.to_string(), NeededFunction::Free);
            //                let (stringify, align) = match element {
            //                    Type::Char => (true, 1),
            //                    _ => (false, self.sizes.align(element)),
            //                };
            //                let tmp = self.tmp();
            //                self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
            //                self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
            //                let result = format!(
            //                    "
            //                        unsafe {{
            //                            copy_slice(
            //                                &mut caller,
            //                                memory,
            //                                func_{},
            //                                ptr{tmp}, len{tmp}, {}
            //                            )?
            //                        }}
            //                    ",
            //                    free,
            //                    align,
            //                    tmp = tmp
            //                );
            //                if stringify {
            //                    results.push(format!(
            //                        "String::from_utf8({})
            //                            .map_err(|_| wasmtime::Trap::new(\"invalid utf-8\"))?",
            //                        result
            //                    ));
            //                } else {
            //                    results.push(result);
            //                }
            //            }
            //            None => {
            //                self.needs_borrow_checker = true;
            //                let method = match element {
            //                    Type::Char => "slice_str",
            //                    _ => "slice",
            //                };
            //                let tmp = self.tmp();
            //                self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
            //                self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
            //                let mut slice = format!("_bc.{}(ptr{1}, len{1})?", method, tmp);
            //                if method == "slice" {
            //                    slice = format!("unsafe {{ {} }}", slice);
            //                }
            //                results.push(slice);
            //            }
            //        }
            //    }

            //    Instruction::ListLower { element, realloc } => {
            //        let realloc = realloc.unwrap();
            //        let body = self.blocks.pop().unwrap();
            //        let tmp = self.tmp();
            //        let vec = format!("vec{}", tmp);
            //        let result = format!("result{}", tmp);
            //        let len = format!("len{}", tmp);
            //        self.needs_functions
            //            .insert(realloc.to_string(), NeededFunction::Realloc);
            //        let size = self.sizes.size(element);
            //        let align = self.sizes.align(element);

            //        // first store our vec-to-lower in a temporary since we'll
            //        // reference it multiple times.
            //        self.push_str(&format!("let {} = {};\n", vec, operands[0]));
            //        self.push_str(&format!("let {} = {}.len() as i32;\n", len, vec));

            //        // ... then realloc space for the result in the guest module
            //        self.push_str(&format!(
            //            "let {} = func_{}.call(&mut caller, (0, 0, {} * {}, {}))?;\n",
            //            result, realloc, len, size, align,
            //        ));
            //        self.caller_memory_available = false; // invalidated by call

            //        // ... then consume the vector and use the block to lower the
            //        // result.
            //        self.push_str(&format!(
            //            "for (i, e) in {}.into_iter().enumerate() {{\n",
            //            vec
            //        ));
            //        self.push_str(&format!("let base = {} + (i as i32) * {};\n", result, size));
            //        self.push_str(&body);
            //        self.push_str("}");

            //        results.push(result);
            //        results.push(len);
            //    }

            //    Instruction::ListLift { element, free } => {
            //        let body = self.blocks.pop().unwrap();
            //        let tmp = self.tmp();
            //        let size = self.sizes.size(element);
            //        let align = self.sizes.align(element);
            //        let len = format!("len{}", tmp);
            //        self.push_str(&format!("let {} = {};\n", len, operands[1]));
            //        let base = format!("base{}", tmp);
            //        self.push_str(&format!("let {} = {};\n", base, operands[0]));
            //        let result = format!("result{}", tmp);
            //        self.push_str(&format!(
            //            "let mut {} = Vec::with_capacity({} as usize);\n",
            //            result, len,
            //        ));

            //        self.push_str("for i in 0..");
            //        self.push_str(&len);
            //        self.push_str(" {\n");
            //        self.push_str("let base = ");
            //        self.push_str(&base);
            //        self.push_str(" + i *");
            //        self.push_str(&size.to_string());
            //        self.push_str(";\n");
            //        self.push_str(&result);
            //        self.push_str(".push(");
            //        self.push_str(&body);
            //        self.push_str(");\n");
            //        self.push_str("}\n");
            //        results.push(result);

            //        if let Some(free) = free {
            //            self.push_str(&format!(
            //                "func_{}.call(&mut caller, ({}, {} * {}, {}))?;\n",
            //                free, base, len, size, align,
            //            ));
            //            self.needs_functions
            //                .insert(free.to_string(), NeededFunction::Free);
            //        }
            //    }

            //    Instruction::IterElem => results.push("e".to_string()),

            //    Instruction::IterBasePointer => results.push("base".to_string()),

            //    // Never used due to the call modes that this binding generator
            //    // uses
            //    Instruction::BufferLowerPtrLen { .. } => unreachable!(),
            //    Instruction::BufferLiftHandle { .. } => unimplemented!(),

            //    Instruction::BufferLiftPtrLen { push, ty } => {
            //        let block = self.blocks.pop().unwrap();
            //        self.needs_borrow_checker = true;
            //        let tmp = self.tmp();
            //        self.push_str(&format!("let _ = {};\n", operands[0]));
            //        self.push_str(&format!("let ptr{} = {};\n", tmp, operands[1]));
            //        self.push_str(&format!("let len{} = {};\n", tmp, operands[2]));
            //        if iface.all_bits_valid(ty) {
            //            let method = if *push { "slice_mut" } else { "slice" };
            //            results.push(format!("unsafe {{ _bc.{}(ptr{1}, len{1})? }}", method, tmp));
            //        } else {
            //            let size = self.sizes.size(ty);
            //            let closure = format!("closure{}", tmp);
            //            self.closures.push_str(&format!("let {} = ", closure));
            //            if *push {
            //                self.closures.push_str("|_bc: &mut [u8], e:");
            //                mem::swap(&mut self.closures, &mut self.src);
            //                self.print_ty(iface, ty, TypeMode::Owned);
            //                mem::swap(&mut self.closures, &mut self.src);
            //                self.closures.push_str("| {let base = 0;\n");
            //                self.closures.push_str(&block);
            //                self.closures.push_str("; Ok(()) };\n");
            //                results.push(format!(
            //                    "witx_bindgen_wasmtime::imports::PushBuffer::new(
            //                        &mut _bc, ptr{}, len{}, {}, &{})?",
            //                    tmp, tmp, size, closure
            //                ));
            //            } else {
            //                self.closures.push_str("|_bc: &[u8]| { let base = 0;Ok(");
            //                self.closures.push_str(&block);
            //                self.closures.push_str(") };\n");
            //                results.push(format!(
            //                    "witx_bindgen_wasmtime::imports::PullBuffer::new(
            //                        &mut _bc, ptr{}, len{}, {}, &{})?",
            //                    tmp, tmp, size, closure
            //                ));
            //            }
            //        }
            //    }

            //    Instruction::BufferLowerHandle { push, ty } => {
            //        let block = self.blocks.pop().unwrap();
            //        let size = self.sizes.size(ty);
            //        let tmp = self.tmp();
            //        let handle = format!("handle{}", tmp);
            //        let closure = format!("closure{}", tmp);
            //        self.needs_buffer_transaction = true;
            //        if iface.all_bits_valid(ty) {
            //            let method = if *push { "push_out_raw" } else { "push_in_raw" };
            //            self.push_str(&format!(
            //                "let {} = unsafe {{ buffer_transaction.{}({}) }};\n",
            //                handle, method, operands[0],
            //            ));
            //        } else if *push {
            //            self.closures.push_str(&format!(
            //                "let {} = |memory: &wasmtime::Memory, base: i32| {{
            //                    Ok(({}, {}))
            //                }};\n",
            //                closure, block, size,
            //            ));
            //            self.push_str(&format!(
            //                "let {} = unsafe {{ buffer_transaction.push_out({}, &{}) }};\n",
            //                handle, operands[0], closure,
            //            ));
            //        } else {
            //            let start = self.src.len();
            //            self.print_ty(iface, ty, TypeMode::AllBorrowed("'_"));
            //            let ty = self.src[start..].to_string();
            //            self.src.truncate(start);
            //            self.closures.push_str(&format!(
            //                "let {} = |memory: &wasmtime::Memory, base: i32, e: {}| {{
            //                    {};
            //                    Ok({})
            //                }};\n",
            //                closure, ty, block, size,
            //            ));
            //            self.push_str(&format!(
            //                "let {} = unsafe {{ buffer_transaction.push_in({}, &{}) }};\n",
            //                handle, operands[0], closure,
            //            ));
            //        }
            //        results.push(format!("{}", handle));
            //    }
            // Instruction::CallWasm {
            //     module: _,
            //     name,
            //     sig,
            // } => {
            //     // if sig.results.len() > 0 {
            //     //     let tmp = self.tmp();
            //     //     self.push_str("let (");
            //     //     for i in 0..sig.results.len() {
            //     //         let arg = format!("result{}_{}", tmp, i);
            //     //         self.push_str(&arg);
            //     //         self.push_str(",");
            //     //         results.push(arg);
            //     //     }
            //     //     self.push_str(") = ");
            //     // }
            //     self.src.js("obj.");
            //     self.src.js(&name.to_snake_case());
            //     self.src.js("(");
            //     self.src.js(&operands.join(", "));
            //     self.src.js(");");
            //     // self.push_str(");");
            //     // self.after_call = true;
            //     // self.caller_memory_available = false; // invalidated by call
            // }
            Instruction::CallInterface { module: _, func } => {
                if func.results.len() > 0 {
                    if func.results.len() == 1 {
                        self.src.js("const ret = ");
                        results.push("ret".to_string());
                    } else if func.results.iter().any(|p| p.0.is_empty()) {
                        self.src.js("const [");
                        for i in 0..func.results.len() {
                            if i > 0 {
                                self.src.js(", ")
                            }
                            let name = format!("ret{}", i);
                            self.src.js(&name);
                            results.push(name);
                        }
                        self.src.js("] = ");
                    } else {
                        self.src.js("const {");
                        for (i, (name, _)) in func.results.iter().enumerate() {
                            if i > 0 {
                                self.src.js(", ")
                            }
                            self.src.js(name);
                            results.push(name.clone());
                        }
                        self.src.js("} = ");
                    }
                }
                self.src.js("obj.");
                self.src.js(&func.name.to_snake_case());
                self.src.js("(");
                self.src.js(&operands.join(", "));
                self.src.js(");\n");
            }

            Instruction::Return { amt } => match amt {
                0 => {}
                1 => self.src.js(&format!("return {};\n", operands[0])),
                _ => panic!(),
            },

            //    Instruction::I32Load { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_load = true;
            //        results.push(format!(
            //            "{}.load({} + {}, [0u8; 4], i32::from_le_bytes)?",
            //            mem, operands[0], offset,
            //        ));
            //    }
            //    Instruction::I32Load8U { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_load = true;
            //        results.push(format!(
            //            "i32::from({}.load({} + {}, [0u8; 1], u8::from_le_bytes)?)",
            //            mem, operands[0], offset,
            //        ));
            //    }
            //    Instruction::I32Load8S { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_load = true;
            //        results.push(format!(
            //            "i32::from({}.load({} + {}, [0u8; 1], i8::from_le_bytes)?)",
            //            mem, operands[0], offset,
            //        ));
            //    }
            //    Instruction::I32Load16U { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_load = true;
            //        results.push(format!(
            //            "i32::from({}.load({} + {}, [0u8; 2], u16::from_le_bytes)?)",
            //            mem, operands[0], offset,
            //        ));
            //    }
            //    Instruction::I32Load16S { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_load = true;
            //        results.push(format!(
            //            "i32::from({}.load({} + {}, [0u8; 2], i16::from_le_bytes)?)",
            //            mem, operands[0], offset,
            //        ));
            //    }
            //    Instruction::I64Load { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_load = true;
            //        results.push(format!(
            //            "{}.load({} + {}, [0u8; 8], i64::from_le_bytes)?",
            //            mem, operands[0], offset,
            //        ));
            //    }
            //    Instruction::F32Load { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_load = true;
            //        results.push(format!(
            //            "{}.load({} + {}, [0u8; 4], f32::from_le_bytes)?",
            //            mem, operands[0], offset,
            //        ));
            //    }
            //    Instruction::F64Load { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_load = true;
            //        results.push(format!(
            //            "{}.load({} + {}, [0u8; 8], f64::from_le_bytes)?",
            //            mem, operands[0], offset,
            //        ));
            //    }
            Instruction::I32Store { offset } => self.store("setInt32", *offset, operands),
            Instruction::I64Store { offset } => self.store("setBigInt64", *offset, operands),

            //    Instruction::I32Store { offset }
            //    | Instruction::I64Store { offset }
            //    | Instruction::F32Store { offset }
            //    | Instruction::F64Store { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_store = true;
            //        self.push_str(&format!(
            //            "{}.store({} + {}, &({}).to_le_bytes())?;\n",
            //            mem, operands[1], offset, operands[0]
            //        ));
            //    }
            //    Instruction::I32Store8 { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_store = true;
            //        self.push_str(&format!(
            //            "{}.store({} + {}, &(({}) as u8).to_le_bytes())?;\n",
            //            mem, operands[1], offset, operands[0]
            //        ));
            //    }
            //    Instruction::I32Store16 { offset } => {
            //        let mem = self.memory_src();
            //        self.needs_store = true;
            //        self.push_str(&format!(
            //            "{}.store({} + {}, &(({}) as u16).to_le_bytes())?;\n",
            //            mem, operands[1], offset, operands[0]
            //        ));
            //    }

            // Instruction::Witx { instr } => match instr {
            //     WitxInstruction::PointerFromI32 { .. }
            //     | WitxInstruction::ConstPointerFromI32 { .. } => top_as("u32"),
            //     i => unimplemented!("{:?}", i),
            // },
            i => unimplemented!("{:?}", i),
        }
    }
}

#[derive(Default)]
struct Source {
    js: String,
    js_level: usize,
    ts: String,
    ts_level: usize,
}

impl Source {
    fn js(&mut self, s: &str) {
        Source::push(&mut self.js, &mut self.js_level, s)
    }
    fn ts(&mut self, s: &str) {
        Source::push(&mut self.ts, &mut self.ts_level, s)
    }

    fn push(dst: &mut String, level: &mut usize, src: &str) {
        let lines = src.lines().map(str::trim).collect::<Vec<_>>();
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("}") {
                dst.pop();
                dst.pop();
            }
            dst.push_str(line);
            if line.ends_with('{') {
                *level += 1;
            } else if line.starts_with('}') {
                *level -= 1;
            }
            if i != lines.len() - 1 || src.ends_with("\n") {
                Source::newline(dst, level);
            }
        }
    }

    fn newline(dst: &mut String, level: &mut usize) {
        dst.push_str("\n");
        for i in 0..*level {
            dst.push_str("  ");
        }
    }
}
