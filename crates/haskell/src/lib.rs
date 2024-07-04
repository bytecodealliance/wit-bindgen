use abi::{AbiVariant, WasmType};
use anyhow::Result;
use heck::{ToLowerCamelCase as _, ToSnakeCase as _, ToUpperCamelCase as _};
use indexmap::{IndexMap, IndexSet};
use wit_bindgen_core::abi::{
    call, guest_export_needs_post_return, post_return, Bindgen, Bitcast, Instruction, LiftLower,
};
use wit_bindgen_core::{dealias, Direction, Files, Source, WorldGenerator};

use wit_parser::*;

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        let mut r = Haskell::default();
        r.opts = self.clone();
        Box::new(r)
    }
}

#[derive(Default)]
struct Module {
    funcs_imp: Source,
    funcs_exp: Source,
    tydefs: IndexSet<String>,
    user: Source,
    docs: Option<String>,
    imports_exports: bool,
}

#[derive(Default)]
pub struct Haskell {
    modules: IndexMap<String, Module>,
    c_header: String,
    opts: Opts,
}

impl WorldGenerator for Haskell {
    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        iface: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let iface = &resolve.interfaces[iface];
        let iname = if let WorldKey::Name(name) = name {
            name.clone()
        } else {
            iface.name.clone().unwrap()
        };
        let module = if let Some(module) = self.modules.get_mut(&iname) {
            module
        } else {
            self.modules.insert(iname.clone(), Default::default());
            self.modules.get_mut(&iname).unwrap()
        };
        module.docs = iface.docs.contents.clone();
        for (name, ty) in &iface.types {
            module.tydefs.insert(gen_typedef(resolve, name, *ty));
        }
        for (_name, func) in &iface.functions {
            module.funcs_imp.push_str(&gen_func_core(
                resolve,
                func,
                &iname,
                AbiVariant::GuestImport,
            ));
            module.funcs_imp.push_str("\n");
            module.funcs_imp.push_str(&gen_func(resolve, &func, &iname));
            self.c_header
                .push_str(&gen_func_c(resolve, func, &iname, Direction::Import));
        }
        Ok(())
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        iface: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let iface = &resolve.interfaces[iface];
        let iname = if let WorldKey::Name(name) = name {
            name.clone()
        } else {
            iface.name.clone().unwrap()
        };
        let module = if let Some(module) = self.modules.get_mut(&iname) {
            module
        } else {
            self.modules.insert(iname.clone(), Default::default());
            self.modules.get_mut(&iname).unwrap()
        };
        module.docs = iface.docs.contents.clone();
        for (name, ty) in &iface.types {
            module.tydefs.insert(gen_typedef(resolve, name, *ty));
        }
        if !iface.functions.is_empty() {
            module.imports_exports = true;
        }
        for (_name, func) in &iface.functions {
            module.funcs_exp.push_str("\n");
            module.funcs_exp.push_str(&gen_func_core(
                resolve,
                func,
                &iname,
                AbiVariant::GuestExport,
            ));
            module.user.push_str(&gen_func_placeholder(resolve, func));
            if guest_export_needs_post_return(resolve, func) {
                module
                    .funcs_exp
                    .push_str(&gen_func_post_return(resolve, func, &iname));
                module.funcs_exp.push_str("\n");
                self.c_header
                    .push_str(&gen_func_c_post_return(resolve, func, &iname));
            }
            self.c_header
                .push_str(&gen_func_c(resolve, func, &iname, Direction::Export));
        }
        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let world = &resolve.worlds[world];
        let module = if let Some(module) = self.modules.get_mut(&world.name) {
            module
        } else {
            self.modules.insert(world.name.clone(), Default::default());
            self.modules.get_mut(&world.name).unwrap()
        };
        module.docs = world.docs.contents.clone();
        for (_name, func) in funcs {
            module.funcs_imp.push_str(&gen_func_core(
                resolve,
                func,
                &world.name,
                AbiVariant::GuestImport,
            ));
            module.funcs_imp.push_str("\n");
            module
                .funcs_imp
                .push_str(&gen_func(resolve, func, &world.name));
            module.funcs_imp.push_str("\n");
            self.c_header
                .push_str(&gen_func_c(resolve, func, &world.name, Direction::Import));
        }
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let world = &resolve.worlds[world];
        let module = if let Some(module) = self.modules.get_mut(&world.name) {
            module
        } else {
            self.modules.insert(world.name.clone(), Default::default());
            self.modules.get_mut(&world.name).unwrap()
        };
        if !funcs.is_empty() {
            module.imports_exports = true;
        }
        module.docs = world.docs.contents.clone();
        for (_name, func) in funcs {
            module.funcs_exp.push_str(&gen_func_core(
                resolve,
                func,
                &world.name,
                AbiVariant::GuestExport,
            ));
            module.funcs_exp.push_str("\n");
            module.user.push_str(&gen_func_placeholder(resolve, func));
            if guest_export_needs_post_return(resolve, func) {
                module
                    .funcs_exp
                    .push_str(&gen_func_post_return(resolve, func, &world.name));
                module.funcs_exp.push_str("\n");
                self.c_header
                    .push_str(&gen_func_c_post_return(resolve, func, &world.name));
            }
            self.c_header
                .push_str(&gen_func_c(resolve, func, &world.name, Direction::Export));
        }
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let world = &resolve.worlds[world];
        let module = if let Some(module) = self.modules.get_mut(&world.name) {
            module
        } else {
            self.modules.insert(world.name.clone(), Default::default());
            self.modules.get_mut(&world.name).unwrap()
        };
        module.docs = world.docs.contents.clone();
        module.tydefs.insert(
            types
                .iter()
                .map(|(name, id)| gen_typedef(resolve, name, *id))
                .collect::<Vec<String>>()
                .join("\n"),
        );
    }

    fn finish(&mut self, _resolve: &Resolve, _world: WorldId, files: &mut Files) -> Result<()> {
        for (name, module) in self.modules.iter_mut() {
            let name = upper_ident(name);
            if !module.funcs_imp.is_empty() {
                let contents = gen_module(
                    &name,
                    &module.funcs_imp,
                    ModuleKind::Imports {
                        imports_types: !module.tydefs.is_empty(),
                    },
                    &module.docs,
                );
                files.push(&format!("{}/Imports.hs", name.replace('.', "/")), &contents);
            }
            if !module.funcs_exp.is_empty() {
                let contents = gen_module(
                    &name,
                    &module.funcs_exp,
                    ModuleKind::Exports {
                        imports_types: !module.tydefs.is_empty(),
                    },
                    &module.docs,
                );
                files.push(&format!("{}/Exports.hs", name.replace('.', "/")), &contents);
            }
            if !module.tydefs.is_empty() {
                let contents = gen_module(
                    &name,
                    &module
                        .tydefs
                        .iter()
                        .cloned()
                        .collect::<Vec<String>>()
                        .join("\n"),
                    ModuleKind::Types,
                    &module.docs,
                );
                files.push(&format!("{}/Types.hs", name.replace('.', "/")), &contents);
            }
            let user = gen_module(
                &name,
                &module.user,
                ModuleKind::User {
                    imports_types: !module.tydefs.is_empty(),
                    imports_imports: !module.funcs_imp.is_empty(),
                },
                &module.docs,
            );
            files.push(&format!("{name}.hs"), &user);
        }
        let c_header = format!("#include <stdint.h>\n\n{}", self.c_header);
        files.push("bg_foreign.h", c_header.as_bytes());
        Ok(())
    }
}

enum ModuleKind {
    Imports {
        imports_types: bool,
    },
    Exports {
        imports_types: bool,
    },
    Types,
    User {
        imports_types: bool,
        imports_imports: bool,
    },
}

fn gen_module(name: &str, src: &str, module_kind: ModuleKind, docs: &Option<String>) -> Vec<u8> {
    let module_name = match module_kind {
        ModuleKind::Imports { .. } => format!("{name}.Imports"),
        ModuleKind::Exports { .. } => format!("{name}.Exports"),
        ModuleKind::Types => format!("{name}.Types"),
        ModuleKind::User { .. } => name.to_owned(),
    };
    format!(
        "\
{{-# LANGUAGE CApiFFI #-}}
-- Generated by wit-bindgen.

{}
module {module_name} where

import Data.Word;
import Data.Int;
import Data.Char;
import Data.Bits;
import Data.Text hiding (length, unpack, pack, zip);
import Data.Text.Encoding;
import Data.ByteString hiding (length, zip);
import GHC.Float;
import Foreign.Ptr;
import Foreign.Storable;
import Foreign.Marshal.Array;
import Foreign.Marshal.Alloc;

{}
{}
{}
",
        if let Some(docs) = docs {
            docs.lines()
                .map(|line| format!("-- {line}\n"))
                .collect::<String>()
        } else {
            "".to_owned()
        },
        match module_kind {
            ModuleKind::Imports {
                imports_types: true,
            }
            | ModuleKind::Exports {
                imports_types: true,
            }
            | ModuleKind::User {
                imports_types: true,
                ..
            } => format!("import {name}.Types;\n"),
            _ => "".to_owned(),
        },
        match module_kind {
            ModuleKind::Exports { .. } => format!("import qualified {name};\n"),
            ModuleKind::User {
                imports_imports: true,
                ..
            } => format!("import qualified {name}.Imports;\n"),
            _ => "".to_owned(),
        },
        src.to_string()
    )
    .as_bytes()
    .to_owned()
}

struct HsFunc<'a> {
    dual_func: &'a str,
    params: Vec<String>,
    blocks: Vec<Source>,
    var_count: usize,
    size_align: SizeAlign,
    variant: AbiVariant,
}

impl<'a> HsFunc<'a> {
    fn var(&mut self) -> String {
        self.var_count += 1;
        format!("bg_v{}", self.var_count - 1)
    }
    fn vars(&mut self, amount: usize) -> Vec<String> {
        (0..amount).map(|_| self.var()).collect()
    }
}

fn gen_typedef(resolve: &Resolve, name: &str, id: TypeId) -> String {
    let mut src = String::new();
    let ty = &resolve.types[id];
    if let Some(docs) = &ty.docs.contents {
        src.push_str("\n");
        src.push_str(
            &docs
                .lines()
                .map(|line| format!("-- {line}\n"))
                .collect::<String>(),
        );
    }
    match &ty.kind {
        TypeDefKind::Record(record) => {
            let record_name = upper_ident(name);
            src.push_str(&format!(
                "data {record_name} = {record_name} {{ {} }};\n",
                record
                    .fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{} :: {}",
                            lower_ident(&[name, &field.name].join("-")),
                            ty_name(resolve, false, &field.ty)
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        TypeDefKind::Resource => {
            let resource_name = upper_ident(name);
            src.push_str(&format!(
                "newtype {resource_name} = {resource_name} Word32;\n"
            ));
        }
        TypeDefKind::Handle(_) => todo!(),
        TypeDefKind::Flags(flags) => {
            let flags_name = upper_ident(name);
            src.push_str(&format!(
                "data {flags_name} = {flags_name} {{ {} }};\n",
                flags
                    .flags
                    .iter()
                    .map(|flag| format!("{} :: Bool", lower_ident(&[name, &flag.name].join("-"))))
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        TypeDefKind::Tuple(_) => todo!(),
        TypeDefKind::Variant(var) => {
            let cases = var
                .cases
                .iter()
                .map(|case| {
                    format!(
                        "{} {}",
                        upper_ident(&[name, &case.name].join("-")),
                        if let Some(ty) = case.ty {
                            ty_name(resolve, false, &ty)
                        } else {
                            "".to_owned()
                        }
                    )
                })
                .collect::<Vec<String>>()
                .join(" | ");
            src.push_str(&format!("data {} = {cases};\n", upper_ident(name)))
        }
        TypeDefKind::Enum(enu) => {
            let cases = enu
                .cases
                .iter()
                .map(|case| upper_ident(&[name, &case.name].join("-")))
                .collect::<Vec<String>>()
                .join(" | ");
            src.push_str(&format!("data {} = {cases};\n", upper_ident(name)))
        }
        TypeDefKind::Option(_) => todo!(),
        TypeDefKind::Result(_) => todo!(),
        TypeDefKind::List(_) => todo!(),
        TypeDefKind::Future(_) => todo!(),
        TypeDefKind::Stream(_) => todo!(),
        TypeDefKind::Type(ty) => {
            src.push_str(&format!(
                "type {} = {};\n",
                upper_ident(name),
                ty_name(resolve, false, ty)
            ));
        }
        TypeDefKind::Unknown => todo!(),
    }
    src
}

impl<'a> Bindgen for HsFunc<'a> {
    type Operand = String;

    fn emit(
        &mut self,
        resolve: &Resolve,
        inst: &Instruction<'_>,
        operands: &mut Vec<Self::Operand>,
        results: &mut Vec<Self::Operand>,
    ) {
        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(val.to_string()),
            Instruction::Bitcasts { casts } => results.extend(
                operands
                    .iter()
                    .zip(casts.iter())
                    .map(|(op, cast)| bitcast(op, cast))
                    .collect::<Vec<String>>(),
            ),
            Instruction::ConstZero { tys } => results.extend(tys.iter().map(|ty| {
                match ty {
                    WasmType::I32 => "(0 :: Word32)",
                    WasmType::I64 => "(0 :: Word64)",
                    WasmType::F32 => "(0.0 :: Float)",
                    WasmType::F64 => "(0.0 :: Double)",
                    WasmType::Pointer => "(0 :: Word32)",
                    WasmType::PointerOrI64 => "(0 :: Word64)",
                    WasmType::Length => "(0 :: Word32)",
                }
                .to_owned()
            })),
            Instruction::I32Load { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Word32 -> IO Word32) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                    operands[0]
                ));
                results.push(var);
            }
            Instruction::I32Load8U { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Word8 -> IO Word8) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n", operands[0]
                ));
                results.push(format!("((fromIntegral :: Word8 -> Word32) {var})"));
            }
            Instruction::I32Load8S { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Int8 -> IO Int8) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                     operands[0]
                ));
                results.push(format!("((fromIntegral :: Int8 -> Word32) {var})"));
            }
            Instruction::I32Load16U { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Word16 -> IO Word16) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                     operands[0]
                ));
                results.push(format!("((fromIntegral :: Word16 -> Word32) {var})"));
            }
            Instruction::I32Load16S { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Int16 -> IO Int16) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                    operands[0]
                ));
                results.push(format!("((fromIntegral :: Int16 -> Word32) {var})"));
            }
            Instruction::I64Load { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Word64 -> IO Word64) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                  operands[0]
                ));
                results.push(var);
            }
            Instruction::F32Load { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Float -> IO Float) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                    operands[0]
                ));
                results.push(var);
            }
            Instruction::F64Load { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Double -> IO Double) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                    operands[0]
                ));
                results.push(var);
            }
            Instruction::PointerLoad { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Word32 -> IO Word32) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                    operands[0]
                ));
                results.push(var);
            }
            Instruction::LengthLoad { offset } => {
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peek :: Ptr Word32 -> IO Word32) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))));\n",
                    operands[0]
                ));
                results.push(var);
            }
            Instruction::I32Store { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word32 -> Word 32 -> IO ()) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset})))) {};\n",
                    operands[1], operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word8 -> Word8 -> IO ()) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset})))) ((fromIntegral :: Word32 -> Word8) {});\n",
                    operands[1], operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word16 -> Word16 -> IO ()) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset})))) ((fromIntegral :: Word32 -> Word16) {});\n",
                    operands[1], operands[0]
                ));
            }
            Instruction::I64Store { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word64 -> Word64 -> IO ()) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset})))) {};\n",
                    operands[1], operands[0]
                ));
            }
            Instruction::F32Store { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Float -> Float -> IO ()) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset})))) {};\n",
                    operands[1], operands[0]
                ));
            }
            Instruction::F64Store { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Double -> Double -> IO ()) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset})))) {};\n",
                    operands[1], operands[0]
                ));
            }
            Instruction::PointerStore { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word32 -> Word32 -> IO ()) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset}))))  {};\n",
                    operands[1], operands[0]
                ));
            }
            Instruction::LengthStore { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word32 -> Word32 -> IO ()) (wordPtrToPtr (WordPtr (fromIntegral ({} + {offset})))) {};\n",
                    operands[1], operands[0]
                ));
            }
            Instruction::I32FromChar => results.push(format!(
                "((fromIntegral :: Int -> Word32) (ord {}))",
                operands[0]
            )),
            Instruction::I64FromU64 => results.push(operands[0].clone()),
            Instruction::I64FromS64 => results.push(format!(
                "((fromIntegral :: Int64 -> Word64) {})",
                operands[0]
            )),
            Instruction::I32FromU32 => results.push(operands[0].clone()),
            Instruction::I32FromS32 => results.push(format!(
                "((fromIntegral :: Int32 -> Word32) {})",
                operands[0]
            )),
            Instruction::I32FromU16 => results.push(format!(
                "((fromIntegral :: Word16 -> Word32) {})",
                operands[0]
            )),
            Instruction::I32FromS16 => results.push(format!(
                "((fromIntegral :: Int16 -> Word32) {})",
                operands[0]
            )),
            Instruction::I32FromU8 => results.push(format!(
                "((fromIntegral :: Word8 -> Word32) {})",
                operands[0]
            )),
            Instruction::I32FromS8 => results.push(format!(
                "((fromIntegral :: Int8 -> Word32) {})",
                operands[0]
            )),
            Instruction::CoreF32FromF32 | Instruction::CoreF64FromF64 => {
                results.push(operands[0].clone())
            }
            Instruction::S8FromI32 => results.push(format!(
                "((fromIntegral :: Word32 -> Int8) {})",
                operands[0]
            )),
            Instruction::U8FromI32 => results.push(format!(
                "((fromIntegral :: Word32 -> Word8) {})",
                operands[0]
            )),
            Instruction::S16FromI32 => results.push(format!(
                "((fromIntegral :: Word32 -> Int16) {})",
                operands[0]
            )),
            Instruction::U16FromI32 => results.push(format!(
                "((fromIntegral :: Word32 -> Word16) {})",
                operands[0]
            )),
            Instruction::S32FromI32 => results.push(format!(
                "((fromIntegral :: Word32 -> Int32) {})",
                operands[0]
            )),
            Instruction::U32FromI32 => results.push(operands[0].clone()),
            Instruction::S64FromI64 => results.push(format!(
                "((fromIntegral :: Word64 -> Int64) {})",
                operands[0]
            )),
            Instruction::U64FromI64 => results.push(operands[0].clone()),
            Instruction::CharFromI32 => results.push(format!("(chr {})", operands[0])),
            Instruction::F32FromCoreF32 | Instruction::F64FromCoreF64 => {
                results.push(operands[0].clone())
            }
            Instruction::BoolFromI32 => results.push(format!("({} /= 0)", operands[0])),
            Instruction::I32FromBool => results.push(format!(
                "(if {} then (1 :: Word32) else (0 :: Word32))",
                operands[0]
            )),
            Instruction::ListCanonLower { element, realloc } => {
                let list = operands[0].clone();
                let ptr = self.var();
                let len = self.var();
                let current_block = self.blocks.last_mut().unwrap();
                current_block.push_str(&format!("let {{ {len} = length {list} }};\n"));
                current_block.push_str(&format!(
                    "{ptr} <- (callocBytes :: Int -> IO (Ptr ({}))) {len};\n",
                    ty_name(resolve, false, element)
                ));
                current_block.push_str(&format!("pokeArray {ptr} {list};\n",));
                results.extend([
                    format!("((fromIntegral :: WordPtr -> Word32) (ptrToWordPtr {ptr}))"),
                    format!("((fromIntegral :: Int -> Word32) {len})"),
                ]);
            }
            Instruction::StringLower { realloc } => {
                let ptr = self.var();
                let len = self.var();
                let current_block = self.blocks.last_mut().unwrap();
                current_block.push_str(&format!(
                    "let {{ bg_tmp = unpack (encodeUtf8 {}) }};\n",
                    operands[0]
                ));
                current_block.push_str(&format!("let {{ {len} = length bg_tmp }};\n"));
                current_block.push_str(&format!(
                    "{ptr} <- (callocBytes :: Int -> IO (Ptr Word8)) {len};\n"
                ));
                current_block.push_str(&format!("pokeArray {ptr} bg_tmp;\n"));
                results.extend([
                    format!("((fromIntegral :: WordPtr -> Word32) (ptrToWordPtr {ptr}))"),
                    format!("((fromIntegral :: Int -> Word32) {len})"),
                ]);
            }
            Instruction::ListLower { element, realloc } => {
                let size = self.size_align.size(element);
                let list = operands[0].clone();
                let block = self.blocks.pop().unwrap();
                let list_len = self.var();
                let list_ptr = self.var();
                let current_block = self.blocks.last_mut().unwrap();
                current_block.push_str(&format!("let {{ {list_len} = length {list} }};\n",));
                current_block.push_str(&format!(
                    "{list_ptr} <- (callocBytes :: Int -> IO (Ptr Word8)) ({list_len} * {});\n",
                    size
                ));
                let ptr_as_word32 =
                    format!("((fromIntegral :: WordPtr -> Word32) (ptrToWordPtr {list_ptr}))");
                current_block.push_str(&format!(
                    "mapM_ (\\(bg_base_ptr, bg_elem) -> do {{\n{}return bg_v\n}}) (zip (if {list_len} == 0 then [] else enumFromThenTo {ptr_as_word32} ({ptr_as_word32} + {size}) ((fromIntegral {list_len}) * {size} + {ptr_as_word32} - {size})) {list});\n",
                    block.to_string()
                ));
                results.extend([
                    ptr_as_word32,
                    format!("((fromIntegral :: Int -> Word32) {list_len})"),
                ]);
            }
            Instruction::ListCanonLift { element, ty } => {
                let ty = ty_name(resolve, false, element);
                let ptr = operands[0].clone();
                let len = operands[1].clone();
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- (peekArray :: Int -> Ptr {ty} -> IO [{ty}]) (fromIntegral {len}) (wordPtrToPtr (WordPtr (fromIntegral {ptr})));\n"
                ));
                results.push(var);
            }
            Instruction::StringLift => {
                let ptr = operands[0].clone();
                let len = operands[1].clone();
                let var = self.var();
                let current_block = self.blocks.last_mut().unwrap();
                current_block.push_str(&format!("bg_tmp <- (peekArray :: Int -> Ptr Word8 -> IO [Word8]) (fromIntegral {len}) (wordPtrToPtr (WordPtr (fromIntegral {ptr})));\n"));
                current_block.push_str(&format!("let {{ {var} = decodeUtf8 (pack bg_tmp) }};\n"));
                results.push(var);
            }
            Instruction::ListLift { element, ty } => {
                let size = self.size_align.size(element);
                let ptr = operands[0].clone();
                let len = operands[1].clone();
                let block = self.blocks.pop().unwrap();
                let var = self.var();
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!(
                        "{var} <- mapM (\\bg_base_ptr -> do {{\n{}return bg_v\n}}) (if {len} == 0 then [] else enumFromThenTo {ptr} ({ptr} + {size}) ({len} * {size} + {ptr} - {size}));\n",
                        block.to_string()
                    ));
                results.push(var);
            }
            Instruction::IterElem { element } => {
                results.push("bg_elem".to_owned());
            }
            Instruction::IterBasePointer => {
                results.push("bg_base_ptr".to_owned());
            }
            Instruction::RecordLower { record, name, ty } => {
                results.extend(record.fields.iter().map(|field| {
                    format!(
                        "({} {})",
                        lower_ident(&format!("{name}-{}", field.name)),
                        operands[0]
                    )
                }));
            }
            Instruction::RecordLift { record, name, ty } => {
                let fields = record
                    .fields
                    .iter()
                    .zip(operands)
                    .map(|(field, op)| {
                        format!("{} = {op}", lower_ident(&format!("{name}-{}", field.name)))
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                results.push(format!("({} {{ {} }})", upper_ident(name), fields));
            }
            Instruction::HandleLower { handle, name, ty } => todo!(),
            Instruction::HandleLift { handle, name, ty } => todo!(),
            Instruction::TupleLower { tuple, ty } => {
                let fields = self.vars(tuple.types.len());
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "let {{ ({}) = {} }};\n",
                    fields.join(", "),
                    operands[0]
                ));
                results.extend(fields);
            }
            Instruction::TupleLift { tuple, ty } => {
                results.push(format!("({})", operands.join(", ")));
            }
            Instruction::FlagsLower { flags, name, ty } => match flags.repr() {
                FlagsRepr::U8 | FlagsRepr::U16 | FlagsRepr::U32(1) => {
                    let rep_ty = match flags.repr() {
                        FlagsRepr::U8 => "Word8",
                        FlagsRepr::U16 => "Word16",
                        FlagsRepr::U32(_) => "Word32",
                    };
                    results.push(format!(
                    "((0 :: {rep_ty}) .|. ({}))",
                    flags
                        .flags
                        .iter()
                        .enumerate()
                        .map(|(i, flag)| {
                            let field = lower_ident(&[*name, &flag.name].join("-"));
                            let mask = 1 << i;
                            format!("(if ({field} {}) then ({mask} :: {rep_ty}) else (0 :: {rep_ty}))", operands[0])
                        })
                        .collect::<Vec<String>>()
                        .join(" .|. ")
                ))
                }
                _ => todo!(),
            },
            Instruction::FlagsLift { flags, name, ty } => {
                results.push(format!(
                    "({} {{ {} }})",
                    upper_ident(name),
                    flags
                        .flags
                        .iter()
                        .enumerate()
                        .map(|(i, flag)| {
                            format!(
                                "{} = ({} .&. {}) /= 0",
                                lower_ident(&format!("{name}-{}", flag.name)),
                                operands[0 / 32],
                                1 << i,
                            )
                        })
                        .collect::<Vec<String>>()
                        .join(", ")
                ));
            }
            Instruction::VariantPayloadName => {
                results.push("bg_payload".to_owned());
            }
            Instruction::VariantLower {
                variant,
                name,
                ty,
                results: types,
            } => {
                let blocks = self.blocks.drain(self.blocks.len() - variant.cases.len()..);
                let cases = variant
                    .cases
                    .iter()
                    .zip(blocks)
                    .map(|(case, block)| {
                        format!(
                            "{}{}{} -> do {{\n{}return bg_v }}",
                            upper_ident(name),
                            upper_ident(&case.name),
                            if case.ty.is_some() { " bg_payload" } else { "" },
                            block.to_string()
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(";\n");
                let vars = self.vars(types.len());
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- case {} of {{\n{cases} }};\n",
                    vars.join(", "),
                    operands[0]
                ));
                results.extend(vars);
            }
            Instruction::VariantLift { variant, name, ty } => {
                let blocks = self.blocks.drain(self.blocks.len() - variant.cases.len()..);
                let cases = variant
                    .cases
                    .iter()
                    .enumerate()
                    .zip(blocks)
                    .map(|((i, case), block)| {
                        format!(
                            "{} -> do {{ {}\n(return ({}{} bg_v)) }}",
                            if i == variant.cases.len() - 1 {
                                "_".to_owned()
                            } else {
                                i.to_string()
                            },
                            block.to_string(),
                            upper_ident(name),
                            upper_ident(&case.name),
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(";\n");
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- case {} of {{\n{cases} }};\n",
                    operands[0]
                ));
                results.push(var);
            }
            Instruction::EnumLower { enum_, name, ty } => {
                let arms = enum_
                    .cases
                    .iter()
                    .enumerate()
                    .map(|(i, case)| {
                        format!(
                            "{}{} -> {i}",
                            upper_ident(name),
                            upper_ident(&format!("{}", case.name))
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(";\n");
                results.push(format!("(case {} of {{\n{arms} }})", operands[0]));
            }
            Instruction::EnumLift { enum_, name, ty } => {
                let arms = enum_
                    .cases
                    .iter()
                    .enumerate()
                    .map(|(i, case)| {
                        format!(
                            "{} -> {}{}",
                            if i == enum_.cases.len() - 1 {
                                "_".to_owned()
                            } else {
                                i.to_string()
                            },
                            upper_ident(name),
                            upper_ident(&case.name)
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(";\n");
                results.push(format!("(case {} of {{\n{arms} }})", operands[0]));
            }
            Instruction::OptionLower {
                payload,
                ty,
                results: types,
            } => {
                let some = self.blocks.pop().unwrap().to_string();
                let none = self.blocks.pop().unwrap().to_string();
                let vars = self.vars(types.len());
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- case {} of {{\nNothing -> do {{\n{none}return bg_v\n}};\nJust bg_payload -> do {{\n{some}return bg_v\n}} }};\n",
                    vars.join(", "),
                    operands[0]
                ));
                results.extend(vars);
            }
            Instruction::OptionLift { payload, ty } => {
                let some = self.blocks.pop().unwrap().to_string();
                let none = self.blocks.pop().unwrap().to_string();
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- case {} of {{\n0 -> (do {{\n{none}return Nothing\n}});\n_ -> (do {{\n{some}return (Just bg_v)\n}}) }};\n",
                    operands[0]
                ));
                results.push(var);
            }
            Instruction::ResultLower {
                result,
                ty,
                results: types,
            } => {
                let err = self.blocks.pop().unwrap().to_string();
                let ok = self.blocks.pop().unwrap().to_string();
                let vars = self.vars(types.len());
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- case {} of {{\nLeft bg_payload -> do {{\n{err}return bg_v\n}};\nRight bg_payload -> do {{\n{ok}return bg_v\n}}\n}};\n",
                    vars.join(", "),
                    operands[0]
                ));
                results.extend(vars);
            }
            Instruction::ResultLift { result, ty } => {
                let err = self.blocks.pop().unwrap().to_string();
                let ok = self.blocks.pop().unwrap().to_string();
                let var = self.var();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{var} <- case {} of {{\n0 -> (do {{\n{ok}return (Right bg_v)\n}});\n_ -> (do {{\n{err}return (Left bg_v)\n}}) }};\n",
                    operands[0]
                ));
                results.push(var);
            }
            Instruction::CallWasm { name, sig } => {
                let vars = self.vars(sig.results.len());
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- ({} {});\n",
                    vars.join(", "),
                    self.dual_func,
                    operands.join(" ")
                ));
                results.extend(vars);
            }
            Instruction::CallInterface { func } => {
                let vars = self.vars(func.results.len());
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- ({} {});\n",
                    vars.join(", "),
                    self.dual_func,
                    operands.join(" ")
                ));
                results.extend(vars);
            }
            Instruction::Return { amt, func } => {
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!("return ({})", operands.join(", ")));
            }
            Instruction::Malloc {
                realloc,
                size,
                align,
            } => todo!(),
            Instruction::GuestDeallocate { size, align } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(free :: Ptr Word8 -> IO ()) (wordPtrToPtr (WordPtr {}));\n",
                    operands[0]
                ));
            }
            Instruction::GuestDeallocateString => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(free :: Ptr Word8 -> IO ()) (wordPtrToPtr (WordPtr {}));\n",
                    operands[0]
                ));
            }
            Instruction::GuestDeallocateList { element } => {
                let block = self.blocks.pop().unwrap();
                let current_block = self.blocks.last_mut().unwrap();
                let size = self.size_align.size(element);
                let ptr = &operands[0];
                let len = &operands[1];
                current_block.push_str(&format!(
                    "mapM_ (\\bg_base_ptr -> do {{\n{}return bg_v\n}}) (if {len} == 0 then [] else enumFromThenTo {ptr} ({ptr} + {size}) ((fromIntegral {len}) * {size} + {ptr} - {size}));\n",
                    block.to_string()
                ));
                current_block
                    .push_str("(free :: Ptr Word8 -> IO ()) (wordPtrToPtr (WordPtr {ptr}));\n");
            }
            Instruction::GuestDeallocateVariant {
                blocks: blocks_count,
            } => {
                let blocks = self.blocks.drain(self.blocks.len() - blocks_count..);
                let cases = blocks
                    .enumerate()
                    .map(|(i, block)| {
                        format!(
                            "{} -> do {{\n{}\n}}",
                            if i == blocks_count - 1 {
                                "_".to_owned()
                            } else {
                                i.to_string()
                            },
                            block.to_string()
                        )
                    })
                    .collect::<Vec<String>>();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "case {} of {{{}}};\n",
                    operands[0],
                    cases.join(";\n")
                ));
            }
        }
    }

    fn return_pointer(&mut self, size: usize, align: usize) -> Self::Operand {
        let current_block = self.blocks.last_mut().unwrap();
        current_block.push_str(&format!(
            "bg_tmp <- (callocBytes :: Int -> IO (Ptr Word8)) {size};\n"
        ));
        current_block.push_str(&format!(
            "let {{ bg_ret_ptr = (fromIntegral :: WordPtr -> Word32) (ptrToWordPtr bg_tmp) }};\n"
        ));
        "bg_ret_ptr".to_owned()
    }

    fn push_block(&mut self) {
        self.blocks.push(Source::default());
    }

    fn finish_block(&mut self, operand: &mut Vec<Self::Operand>) {
        self.blocks
            .last_mut()
            .unwrap()
            .push_str(&format!("let {{ bg_v = ({}) }};\n", operand.join(", ")));
    }

    fn sizes(&self) -> &SizeAlign {
        &self.size_align
    }

    fn is_list_canonical(&self, resolve: &Resolve, element: &Type) -> bool {
        match element {
            Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::S8
            | Type::S16
            | Type::S32
            | Type::S64
            | Type::F32
            | Type::F64
            | Type::Char => true,
            Type::Id(id) => {
                let ty = resolve
                    .types
                    .get(dealias(resolve, *id))
                    .map(|ty| ty.kind.clone());
                if let Some(TypeDefKind::Type(ty)) = ty {
                    self.is_list_canonical(resolve, &ty)
                } else {
                    false
                }
            }
            Type::Bool | Type::String => false,
        }
    }
}

fn bitcast(op: &String, cast: &Bitcast) -> String {
    match cast {
        Bitcast::F32ToI32 => format!("(castFloatToWord32 {op})"),
        Bitcast::F64ToI64 => format!("(castDoubleToWord64 {op})"),
        Bitcast::I32ToI64 => format!("((fromIntegral :: Word32 -> Word64) {op})"),
        Bitcast::F32ToI64 => {
            format!("((fromIntegral :: Word32 -> Word64) (castFloatToWord32 {op}))")
        }
        Bitcast::I32ToF32 => format!("(castWord32ToFloat {op})"),
        Bitcast::I64ToF64 => format!("(castWord64ToDouble {op})"),
        Bitcast::I64ToI32 => format!("((fromIntegral :: Word64 -> Word32) {op})"),
        Bitcast::I64ToF32 => {
            format!("(castWord32ToFloat ((fromIntegral :: Word64 -> Word32) {op}))")
        }
        Bitcast::P64ToI64 => format!("((fromIntegral :: Word64 -> Word64) {op})"),
        Bitcast::I64ToP64 => op.clone(),
        Bitcast::P64ToP => format!("((fromIntegral :: Word64 -> Word32) {op})"),
        Bitcast::PToP64 => format!("((fromIntegral :: Word32 -> Word64) {op})"),
        Bitcast::I32ToP => op.clone(),
        Bitcast::PToI32 => op.clone(),
        Bitcast::PToL => op.clone(),
        Bitcast::LToP => op.clone(),
        Bitcast::I32ToL => op.clone(),
        Bitcast::LToI32 => op.clone(),
        Bitcast::I64ToL => format!("((fromIntegral :: Word64 -> Word32) {op})"),
        Bitcast::LToI64 => format!("((fromIntegral :: Word32 -> Word64) {op})"),
        Bitcast::Sequence(seq) => {
            let [first, second] = &**seq;
            bitcast(&bitcast(op, first), second)
        }
        Bitcast::None => op.clone(),
    }
}

fn gen_func(resolve: &Resolve, func: &Function, ns: &str) -> String {
    let mut src = String::new();
    if let Some(docs) = &func.docs.contents {
        src.push_str("\n");
        src.push_str(
            &docs
                .lines()
                .map(|line| format!("-- {line}\n"))
                .collect::<String>(),
        );
    }
    src.push_str(&format!("{} :: ", func_name(func, None)));
    src.push_str(
        &func
            .params
            .iter()
            .map(|(_name, ty)| format!("{} ->", ty_name(resolve, false, ty)))
            .collect::<Vec<String>>()
            .join(" "),
    );
    src.push_str(" IO ");
    match &func.results {
        Results::Named(results) => {
            src.push_str(&format!(
                "({})",
                results
                    .iter()
                    .map(|(_name, ty)| ty_name(resolve, false, ty))
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        Results::Anon(ty) => {
            let mut name = ty_name(resolve, false, &ty);
            if name.contains(' ') && !name.starts_with('(') && !name.starts_with('[') {
                name = format!("({name})");
            }
            src.push_str(&name);
        }
    }
    let mut size_align = SizeAlign::new(AddressSize::Wasm32);
    size_align.fill(resolve);
    let mut bindgen = HsFunc {
        dual_func: &func_name_foreign(func, ns, Direction::Import, false),
        params: func
            .params
            .iter()
            .map(|(name, _ty)| lower_ident(&name))
            .collect(),
        blocks: vec![Source::default()],
        var_count: 0,
        size_align,
        variant: AbiVariant::GuestImport,
    };
    src.push_str(";\n");
    src.push_str(&format!(
        "{} {} = ",
        func_name(func, None),
        bindgen.params.join(" ")
    ));
    call(
        resolve,
        AbiVariant::GuestImport,
        LiftLower::LowerArgsLiftResults,
        func,
        &mut bindgen,
    );
    src.push_str(&format!("do {{\n{}\n}};\n", &bindgen.blocks[0].to_string()));

    src.push('\n');
    src
}

fn gen_func_core(resolve: &Resolve, func: &Function, ns: &str, variant: AbiVariant) -> String {
    let mut src = String::new();
    let sig = resolve.wasm_signature(variant, func);
    let name_foreign = func_name_foreign(
        func,
        ns,
        if variant == AbiVariant::GuestExport {
            Direction::Export
        } else {
            Direction::Import
        },
        false,
    );
    src.push_str(
        &(if variant == AbiVariant::GuestExport {
            format!("foreign export capi \"{name_foreign}\" {name_foreign} :: ")
        } else {
            format!("foreign import capi \"../bg_foreign.h {name_foreign}\" {name_foreign} :: ")
        }),
    );
    src.push_str(
        &sig.params
            .iter()
            .map(|ty| format!("{} -> ", core_ty_name(*ty)))
            .collect::<Vec<String>>()
            .join(""),
    );
    let results = if sig.results.len() == 1 {
        format!("IO {}", core_ty_name(sig.results[0]))
    } else {
        format!(
            "IO ({})",
            sig.results
                .iter()
                .map(|ty| core_ty_name(*ty))
                .collect::<Vec<String>>()
                .join(", ")
        )
    };
    src.push_str(&results);
    src.push_str(";");
    if variant == AbiVariant::GuestExport {
        let mut size_align = SizeAlign::new(AddressSize::Wasm32);
        size_align.fill(resolve);
        let mut bindgen = HsFunc {
            dual_func: &func_name(func, Some(&ns)),
            params: sig
                .params
                .iter()
                .enumerate()
                .map(|(i, _)| format!("bg_p{i}"))
                .collect(),
            blocks: vec![Source::default()],
            var_count: 0,
            size_align,
            variant,
        };
        src.push('\n');
        src.push_str(&format!("{} {} = ", name_foreign, bindgen.params.join(" ")));
        call(
            resolve,
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
        );
        src.push_str(&format!("do {{\n{}\n}};\n", &bindgen.blocks[0].to_string()));
    }
    src.push('\n');
    src
}

fn gen_func_post_return(resolve: &Resolve, func: &Function, ns: &str) -> String {
    let mut src = String::new();
    src.push('\n');
    let name_foreign = func_name_foreign(func, ns, Direction::Export, true);
    let params = resolve
        .wasm_signature(AbiVariant::GuestExport, func)
        .results;
    src.push_str(&format!(
        "foreign export capi \"{name_foreign}\" {name_foreign} :: {} -> IO ();\n",
        params
            .iter()
            .map(|ty| core_ty_name(*ty))
            .collect::<Vec<String>>()
            .join(" -> ")
    ));
    let params = params
        .iter()
        .enumerate()
        .map(|(i, _)| format!("bg_p{i}"))
        .collect::<Vec<String>>();
    src.push_str(&format!("{name_foreign} {} = ", params.join(" ")));
    let mut size_align = SizeAlign::new(AddressSize::Wasm32);
    size_align.fill(resolve);
    let mut bindgen = HsFunc {
        dual_func: &name_foreign,
        params,
        blocks: vec![Source::default()],
        var_count: 0,
        size_align,
        variant: AbiVariant::GuestExport,
    };
    post_return(resolve, func, &mut bindgen);
    src.push_str(&format!("do {{\n{}\n}};\n", &bindgen.blocks[0].to_string()));
    src
}

fn gen_func_placeholder(resolve: &Resolve, func: &Function) -> String {
    let mut src = String::new();
    src.push('\n');
    let name = lower_ident(&func.name);
    if let Some(docs) = &func.docs.contents {
        src.push_str(
            &docs
                .lines()
                .map(|line| format!("-- {line}\n"))
                .collect::<String>(),
        );
    }
    src.push_str(&format!("{name} :: "));
    for (_, ty) in &func.params {
        src.push_str(&ty_name(resolve, false, &ty));
        src.push_str(" -> ");
    }
    src.push_str("IO ");
    match &func.results {
        Results::Named(params) => {
            src.push_str(&format!(
                "({})",
                params
                    .iter()
                    .map(|(_, ty)| ty_name(resolve, false, ty))
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        Results::Anon(ty) => {
            let mut name = ty_name(resolve, false, &ty);
            if name.contains(' ') && !name.starts_with('(') && !name.starts_with('[') {
                name = format!("({})", name);
            }
            src.push_str(&name);
        }
    }
    src.push_str(&format!(
        "\n{name} {} = undefined\n",
        func.params
            .iter()
            .map(|(name, _)| lower_ident(name))
            .collect::<Vec<String>>()
            .join(" ")
    ));
    src
}

fn gen_func_c(resolve: &Resolve, func: &Function, ns: &str, dir: Direction) -> String {
    let sig = resolve.wasm_signature(
        if dir == Direction::Import {
            AbiVariant::GuestImport
        } else {
            AbiVariant::GuestExport
        },
        func,
    );
    let func_name_foreign = func_name_foreign(func, ns, dir, false);
    let symbol = func.core_export_name(Some(ns));
    let ret_ty = match sig.results.as_slice() {
        [] => "void".to_owned(),
        [ty] => ty_name_c(ty),
        _ => unimplemented!(),
    };
    let params = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("{} bg_p{i}", ty_name_c(ty)))
        .collect::<Vec<String>>()
        .join(", ");
    let vars = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, _)| format!("bg_p{i}"))
        .collect::<Vec<String>>()
        .join(", ");
    if dir == Direction::Import {
        format!(
            "\
{ret_ty} {func_name_foreign}({params}) __attribute__((
  __import_module__(\"\"),
  __import_name__(\"{symbol}\")
));

",
        )
    } else {
        let func_name_export = [ns, &func.name].join("-").to_snake_case();
        format!(
            "\
{ret_ty} {func_name_export}({params}) __attribute__((
  __export_name__(\"{symbol}\")
));
{ret_ty} {func_name_export}({params}) {{
  {}{func_name_foreign}({vars});
}}

",
            if func.results.len() == 0 {
                ""
            } else {
                "return "
            },
        )
    }
}

fn gen_func_c_post_return(resolve: &Resolve, func: &Function, ns: &str) -> String {
    let func_name_foreign = func_name_foreign(func, ns, Direction::Export, true);
    let func_name_export = format!("cabi_post_{}", [ns, &func.name].join("-").to_snake_case());
    let symbol = format!("cabi_post_{}", func.core_export_name(Some(ns)));
    let sig = resolve.wasm_signature(AbiVariant::GuestExport, func);
    let params = sig
        .results
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("{} bg_p{i}", ty_name_c(ty)))
        .collect::<Vec<String>>()
        .join(", ");
    let vars = sig
        .results
        .iter()
        .enumerate()
        .map(|(i, _)| format!("bg_p{i}"))
        .collect::<Vec<String>>()
        .join(", ");
    format!(
        "\
void {func_name_export}({params}) __attribute__((
  __export_name__(\"{symbol}\")
));
void {func_name_export}({params}) {{
  {func_name_foreign}({vars});
}}

"
    )
}

fn ty_name_c(ty: &WasmType) -> String {
    match ty {
        WasmType::I32 => "uint32_t".to_owned(),
        WasmType::I64 => "uint64_t".to_owned(),
        WasmType::F32 => "float".to_owned(),
        WasmType::F64 => "double".to_owned(),
        WasmType::Pointer => "uint32_t".to_owned(),
        WasmType::PointerOrI64 => "uint64_t".to_owned(),
        WasmType::Length => "uint32_t".to_owned(),
    }
}

fn func_name_foreign(func: &Function, ns: &str, dir: Direction, post_return: bool) -> String {
    format!(
        "bg_fn_{}_{}_{}",
        if dir == Direction::Import {
            "imp"
        } else if post_return {
            "post"
        } else {
            "exp"
        },
        upper_ident(ns),
        lower_ident(&func.core_export_name(None))
            .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    )
}

fn func_name(func: &Function, ns: Option<&str>) -> String {
    if let Some(ns) = ns {
        format!("{}.{}", upper_ident(ns), lower_ident(&func.name))
    } else {
        lower_ident(&func.name)
    }
}

fn lower_ident(name: &str) -> String {
    name.to_lower_camel_case()
}

fn upper_ident(name: &str) -> String {
    name.to_upper_camel_case()
}

fn ty_name(resolve: &Resolve, with_ns: bool, ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_owned(),
        Type::U8 => "Word8".to_owned(),
        Type::U16 => "Word16".to_owned(),
        Type::U32 => "Word32".to_owned(),
        Type::U64 => "Word64".to_owned(),
        Type::S8 => "Int8".to_owned(),
        Type::S16 => "Int16".to_owned(),
        Type::S32 => "Int32".to_owned(),
        Type::S64 => "Int64".to_owned(),
        Type::F32 => "Float".to_owned(),
        Type::F64 => "Double".to_owned(),
        Type::Char => "Char".to_owned(),
        Type::String => "Text".to_owned(),
        Type::Id(id) => {
            let ty = &resolve.types[*id];
            let ns: Option<String> = if with_ns {
                match ty.owner {
                    TypeOwner::World(id) => Some(resolve.worlds[id].name.clone()),
                    TypeOwner::Interface(id) => {
                        if let Some(name) = resolve.interfaces[id].name.clone() {
                            Some(name)
                        } else {
                            None
                        }
                    }
                    TypeOwner::None => None,
                }
            } else {
                None
            };
            let ns = ns.map(|n| format!("{}.Types", upper_ident(&n)));
            if let Some(name) = &ty.name {
                if let Some(ns) = ns {
                    format!("{ns}.{}", upper_ident(name))
                } else {
                    upper_ident(name)
                }
            } else {
                match &ty.kind {
                    TypeDefKind::Record(_) => todo!(),
                    TypeDefKind::Resource => todo!(),
                    TypeDefKind::Handle(_) => todo!(),
                    TypeDefKind::Flags(_) => todo!(),
                    TypeDefKind::Tuple(tuple) => {
                        format!(
                            "({})",
                            tuple
                                .types
                                .iter()
                                .map(|ty| { ty_name(resolve, with_ns, ty) })
                                .collect::<Vec<String>>()
                                .join(", ")
                        )
                    }
                    TypeDefKind::Variant(_) => todo!(),
                    TypeDefKind::Enum(_) => todo!(),
                    TypeDefKind::Option(ty) => {
                        format!("Maybe {}", ty_name(resolve, with_ns, ty))
                    }
                    TypeDefKind::Result(result) => {
                        let ok_ty = if let Some(ty) = result.ok {
                            ty_name(resolve, with_ns, &ty)
                        } else {
                            "()".to_owned()
                        };
                        let err_ty = if let Some(ty) = result.err {
                            ty_name(resolve, with_ns, &ty)
                        } else {
                            "()".to_owned()
                        };
                        format!("Either {err_ty} {ok_ty}")
                    }
                    TypeDefKind::List(ty) => {
                        format!("[{}]", ty_name(resolve, with_ns, ty))
                    }
                    TypeDefKind::Future(_) => todo!(),
                    TypeDefKind::Stream(_) => todo!(),
                    TypeDefKind::Type(ty) => ty_name(resolve, with_ns, ty),
                    TypeDefKind::Unknown => todo!(),
                }
            }
        }
    }
}

fn core_ty_name(ty: WasmType) -> String {
    format!(
        "{}",
        match ty {
            abi::WasmType::I32 => "Word32",
            abi::WasmType::I64 => "Word64",
            abi::WasmType::F32 => "Float",
            abi::WasmType::F64 => "Double",
            abi::WasmType::Pointer => "Word32",
            abi::WasmType::PointerOrI64 => "Word64",
            abi::WasmType::Length => "Word32",
        }
    )
}
