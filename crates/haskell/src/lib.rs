use abi::{AbiVariant, WasmType};
use anyhow::Result;
use heck::{ToLowerCamelCase as _, ToUpperCamelCase as _};
use indexmap::{IndexMap, IndexSet};
use wit_bindgen_core;
use wit_bindgen_core::abi::{call, Bindgen, Bitcast, Instruction, LiftLower};
use wit_bindgen_core::{Direction, Files, Source, WorldGenerator};

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
pub struct Module {
    funcs: Source,
    tydefs: IndexSet<String>,
    docs: Option<String>,
    imports_exports: bool,
}

#[derive(Default)]
pub struct Haskell {
    modules: IndexMap<String, Module>,
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
        let iname = upper_ident(None, &iname);
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
        for (name, func) in &iface.functions {
            module.funcs.push_str(&gen_func_core(
                resolve,
                func,
                iface.name.as_deref(),
                AbiVariant::GuestImport,
            ));
            module.funcs.push_str("\n");
            module
                .funcs
                .push_str(&gen_func(resolve, &func, iface.name.as_deref()));
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
        let iname = upper_ident(None, &iname);
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
        for (name, func) in &iface.functions {
            module.funcs.push_str("\n");
            module.funcs.push_str(&gen_func_core(
                resolve,
                func,
                iface.name.as_deref(),
                AbiVariant::GuestExport,
            ));
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
        let world_name = upper_ident(None, &world.name);
        let module = if let Some(module) = self.modules.get_mut(&world_name) {
            module
        } else {
            self.modules.insert(world_name.clone(), Default::default());
            self.modules.get_mut(&world_name).unwrap()
        };
        module.docs = world.docs.contents.clone();
        for (name, func) in funcs {
            module
                .funcs
                .push_str(&gen_func_core(resolve, func, None, AbiVariant::GuestImport));
            module.funcs.push_str("\n");
            module
                .funcs
                .push_str(&gen_func(resolve, func, Some(&world_name)));
            module.funcs.push_str("\n");
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
        let world_name = upper_ident(None, &world.name);
        let module = if let Some(module) = self.modules.get_mut(&world_name) {
            module
        } else {
            self.modules.insert(world_name.clone(), Default::default());
            self.modules.get_mut(&world_name).unwrap()
        };
        if !funcs.is_empty() {
            module.imports_exports = true;
        }
        module.docs = world.docs.contents.clone();
        for (name, func) in funcs {
            module
                .funcs
                .push_str(&gen_func_core(resolve, func, None, AbiVariant::GuestExport));
            module.funcs.push_str("\n");
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
        let world_name: String = upper_ident(None, &world.name);
        let module = if let Some(module) = self.modules.get_mut(&world_name) {
            module
        } else {
            self.modules.insert(world_name.clone(), Default::default());
            self.modules.get_mut(&world_name).unwrap()
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
            let contents = gen_module(
                name,
                &module.funcs,
                module.imports_exports,
                !module.tydefs.is_empty(),
                &module.docs,
            );
            files.push(&format!("{name}.hs"), &contents);
            if module.tydefs.is_empty() {
                continue;
            }
            let contents = gen_module(
                &format!("Types.{name}"),
                &module
                    .tydefs
                    .iter()
                    .map(|m| m.clone())
                    .collect::<Vec<String>>()
                    .join("\n"),
                false,
                false,
                &module.docs,
            );
            files.push(&format!("Types.{name}.hs"), &contents);
        }
        Ok(())
    }
}

fn gen_module(
    name: &str,
    src: &str,
    imports_exports: bool,
    import_types: bool,
    docs: &Option<String>,
) -> Vec<u8> {
    format!(
        "\
-- Generated by wit-bindgen.

{}
module {name} where

import Data.Word;
import Data.Int;
import Data.Bits;
import Data.Text;
import Data.Text.Encoding;
import Data.ByteString;
import GHC.Float;
import Foreign.Ptr;
import Foreign.Storable;
import Foreign.Marshal.Array;

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
        if import_types {
            format!("\nimport Types.{name};\n")
        } else {
            "".to_owned()
        },
        if imports_exports {
            format!("\nimport qualified Exports.{name};\n")
        } else {
            "".to_owned()
        },
        src.to_string()
    )
    .as_bytes()
    .to_owned()
}

struct HsFunc<'a> {
    ns: Option<&'a str>,
    dual_func: String,
    params: Vec<String>,
    blocks: Vec<Source>,
    var_count: usize,
    size_align: SizeAlign,
    variant: AbiVariant,
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
            let record_name = upper_ident(None, name);
            src.push_str(&format!(
                "data {record_name} = {record_name} {{ {} }};\n",
                record
                    .fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{} :: {}",
                            lower_ident(None, &[name, &field.name].join("-")),
                            ty_name(resolve, false, &field.ty)
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        TypeDefKind::Resource => {
            let resource_name = upper_ident(None, name);
            src.push_str(&format!(
                "newtype {resource_name} = {resource_name} Word32;\n"
            ));
        }
        TypeDefKind::Handle(_) => todo!(),
        TypeDefKind::Flags(flags) => {
            let flags_name = upper_ident(None, name);
            src.push_str(&format!(
                "data {flags_name} = {flags_name} {{ {} }};\n",
                flags
                    .flags
                    .iter()
                    .map(|flag| format!(
                        "{} :: Bool",
                        lower_ident(None, &[name, &flag.name].join("-"))
                    ))
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
                        upper_ident(None, &[name, &case.name].join("-")),
                        if let Some(ty) = case.ty {
                            ty_name(resolve, false, &ty)
                        } else {
                            "".to_owned()
                        }
                    )
                })
                .collect::<Vec<String>>()
                .join(" | ");
            src.push_str(&format!("data {} = {cases};\n", upper_ident(None, name)))
        }
        TypeDefKind::Enum(enu) => {
            let cases = enu
                .cases
                .iter()
                .map(|case| upper_ident(None, &[name, &case.name].join("-")))
                .collect::<Vec<String>>()
                .join(" | ");
            src.push_str(&format!("data {} = {cases};\n", upper_ident(None, name)))
        }
        TypeDefKind::Option(ty) => todo!(),
        TypeDefKind::Result(_) => todo!(),
        TypeDefKind::List(_) => todo!(),
        TypeDefKind::Future(_) => todo!(),
        TypeDefKind::Stream(_) => todo!(),
        TypeDefKind::Type(ty) => {
            src.push_str(&format!(
                "type {} = {};\n",
                upper_ident(None, name),
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
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Word32 -> IO Word32) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::I32Load8U { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Word8 -> IO Word8) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!(
                    "((fromIntegral :: Word8 -> Word32) bg_v{})",
                    self.var_count
                ));
                self.var_count += 1;
            }
            Instruction::I32Load8S { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Int8 -> IO Int8) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!(
                    "((fromIntegral :: Int8 -> Word32) bg_v{})",
                    self.var_count
                ));
                self.var_count += 1;
            }
            Instruction::I32Load16U { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Word16 -> IO Word16) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!(
                    "((fromIntegral :: Word16 -> Word32) bg_v{})",
                    self.var_count
                ));
                self.var_count += 1;
            }
            Instruction::I32Load16S { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Int16 -> IO Int16) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!(
                    "((fromIntegral :: Int16 -> Word32) bg_v{})",
                    self.var_count
                ));
                self.var_count += 1;
            }
            Instruction::I64Load { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Word64 -> IO Word64) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::F32Load { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Float -> IO Float) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::F64Load { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Double -> IO Double) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::PointerLoad { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Word32 -> IO Word32) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::LengthLoad { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (peek :: Ptr Word32 -> IO Word32) (wordPtrToPtr (WordPtr ({} + {offset})));\n",
                    self.var_count, operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::I32Store { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word32 -> Word32 -> IO ()) (wordPtrToPtr (WordPtr ({} + {offset}))) {};\n",
                    operands[0], operands[1]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word8 -> Word8 -> IO ()) (wordPtrToPtr (WordPtr ({} + {offset}))) ((fromIntegral :: Word32 -> Word8) {});\n",
                    operands[0], operands[1]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word16 -> Word16 -> IO ()) (wordPtrToPtr (WordPtr ({} + {offset}))) ((fromIntegral :: Word32 -> Word16) {});\n",
                    operands[0], operands[1]
                ));
            }
            Instruction::I64Store { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word64 -> Word64 -> IO ()) (wordPtrToPtr (WordPtr ({} + {offset}))) {};\n",
                    operands[0], operands[1]
                ));
            }
            Instruction::F32Store { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Float -> Float -> IO ()) (wordPtrToPtr (WordPtr ({} + {offset}))) {};\n",
                    operands[0], operands[1]
                ));
            }
            Instruction::F64Store { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Double -> Double -> IO ()) (wordPtrToPtr (WordPtr ({} + {offset}))) {};\n",
                    operands[0], operands[1]
                ));
            }
            Instruction::PointerStore { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word32 -> Word32 -> IO ()) (wordPtrToPtr (WordPtr ({} + {offset})))  {};\n",
                    operands[0], operands[1]
                ));
            }
            Instruction::LengthStore { offset } => {
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "(poke :: Ptr Word32 -> Word32 -> IO ()) (wordPtrToPtr (WordPtr ({} + {offset}))) ((fromIntegral :: Word32 -> Word32) {});\n",
                    operands[0], operands[1]
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
            Instruction::I32FromU32 => results.push(format!(
                "((fromIntegral :: Word32 -> Word64) {})",
                operands[0]
            )),
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
                let ptr: String = format!("bg_v{}", self.var_count);
                let len = format!("bg_v{}", self.var_count + 1);
                self.var_count += 2;
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!("{len} <- length {list};\n"));
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{ptr} <- (callocBytes :: Int -> IO (Ptr [{}])) {len};\n",
                    ty_name(resolve, false, element)
                ));
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!("pokeArray {ptr} {list};\n",));
                results.extend([ptr, len]);
            }
            Instruction::StringLower { realloc } => {
                let ptr: String = format!("bg_v{}", self.var_count);
                let len = format!("bg_v{}", self.var_count + 1);
                self.var_count += 2;
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!("bg_tmp <- unpack (encodeUtf8 {});\n", operands[0]));
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!("{len} <- length bg_tmp;\n"));
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{ptr} <- (callocBytes :: Int -> IO (Ptr Word8)) {len};\n"
                ));
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!("pokeArray {ptr} bg_tmp;\n"));
                results.extend([ptr, len]);
            }
            Instruction::ListLower { element, realloc } => {
                let size = self.size_align.size(element);
                let list = operands[0].clone();
                let block = self.blocks.pop().unwrap();
                let list_len = format!("bg_v{}", self.var_count + 1);
                let list_ptr = format!("bg_v{}", self.var_count);
                self.var_count += 2;
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!("{list_len} <- length {list};\n",));
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "{list_ptr} <- (callocBytes :: Int -> IO (Ptr Word8)) ({list_len} * {});\n",
                    size
                ));
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "mapM_ (\\(bg_base_ptr, bg_elem) -> do {{\n{}\n}}) (zip (enumFromThenTo {list_ptr} ({list_ptr} + {size}) ({list_len} * {size} - {size})) {list});\n",
                    block.to_string()
                ));
                results.extend([list_ptr, list_len]);
            }
            Instruction::ListCanonLift { element, ty } => {
                let ty = ty_name(resolve, false, element);
                let ptr = operands[0].clone();
                let len = operands[1].clone();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- ((peekArray :: Int -> Ptr {ty} -> IO [ty]) {len} (wordPtrToPtr {ptr}));\n",
                    self.var_count
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::StringLift => {
                let ptr = operands[0].clone();
                let len = operands[1].clone();
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!("bg_v{} <- return (decodeUtf8 ((pack :: [Word8] -> ByteString) ((peekArray :: Int -> Ptr Word8 -> IO [Word8]) {len} (wordPtrToPtr {ptr}))));\n", self.var_count));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::ListLift { element, ty } => {
                let size = self.size_align.size(element);
                let ptr = operands[0].clone();
                let len = operands[1].clone();
                let block = self.blocks.pop().unwrap();
                self.blocks
                    .last_mut()
                    .unwrap()
                    .push_str(&format!(
                        "bg_v{} <- mapM (\\bg_base_ptr -> do {{\n{};\nreturn bg_v\n}}) (enumFromThenTo {ptr} ({ptr} + {size}) ({len} * {size} - {size}));\n",
                        self.var_count,
                        block.to_string()
                    ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
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
                        lower_ident(self.ns, &format!("{name}-{}", field.name)),
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
                        format!(
                            "{} = {op}",
                            lower_ident(self.ns, &format!("{name}-{}", field.name))
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                results.push(format!("{{ {} }}", fields));
            }
            Instruction::HandleLower { handle, name, ty } => todo!(),
            Instruction::HandleLift { handle, name, ty } => todo!(),
            Instruction::TupleLower { tuple, ty } => {
                let fields = tuple
                    .types
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("bg_v{}", self.var_count + i))
                    .collect::<Vec<String>>();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- return ({});\n",
                    fields.join(", "),
                    operands[0]
                ));
                self.var_count += fields.len();
                results.extend(fields);
            }
            Instruction::TupleLift { tuple, ty } => {
                results.push(format!("({})", operands.join(", ")));
            }
            Instruction::FlagsLower { flags, name, ty } => todo!(),
            Instruction::FlagsLift { flags, name, ty } => {
                results.push(format!(
                    "({} {{ {} }})",
                    upper_ident(None, name),
                    flags
                        .flags
                        .iter()
                        .enumerate()
                        .map(|(i, flag)| {
                            format!(
                                "{} = ((shiftR {} {i}) (.&.) 1) == 1)",
                                flag.name,
                                operands[0 / 32]
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
                            "{}{}{} -> do {{\n{}; return bg_v }}",
                            upper_ident(None, name),
                            upper_ident(None, &case.name),
                            if case.ty.is_some() { " bg_payload" } else { "" },
                            block.to_string()
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(";\n");
                let vars = types
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("bg_v{}", self.var_count + i))
                    .collect::<Vec<String>>();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- (case {} of {{\n{cases} }});\n",
                    vars.join(", "),
                    operands[0]
                ));
                results.extend(vars);
                self.var_count += types.len();
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
                            "{i} -> do {{ {};\n(return ({}{} bg_v))\n}}\n",
                            block.to_string(),
                            ty_name(resolve, false, &Type::Id(*ty)),
                            upper_ident(None, &case.name),
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(";\n");
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (case {} of {{\n{cases} }})",
                    self.var_count, operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::EnumLower { enum_, name, ty } => {
                let arms = enum_
                    .cases
                    .iter()
                    .enumerate()
                    .map(|(i, case)| {
                        format!(
                            "{}{} -> {i}",
                            ty_name(resolve, false, &Type::Id(*ty)),
                            upper_ident(None, &format!("{}", case.name))
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
                            "{i} -> {}{}",
                            ty_name(resolve, false, &Type::Id(*ty)),
                            upper_ident(None, &case.name)
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(";\n");
                results.push(format!(
                    "(case {} of {{\n{arms};\n_ -> error \"\" }})",
                    operands[0]
                ));
            }
            Instruction::OptionLower {
                payload,
                ty,
                results: types,
            } => {
                let some = self.blocks.pop().unwrap().to_string();
                let none = self.blocks.pop().unwrap().to_string();
                let vars = types
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("bg_v{}", self.var_count + i))
                    .collect::<Vec<String>>();
                self.var_count += vars.len();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- case {} of {{\nNothing -> do {{\n{none}\n}};\nJust bg_payload -> do {{\n{some}\n}} }}\n",
                    vars.join(", "),
                    operands[0]
                ));
                results.extend(vars);
            }
            Instruction::OptionLift { payload, ty } => {
                let some = self.blocks.pop().unwrap().to_string();
                let none = self.blocks.pop().unwrap().to_string();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (case {} of\n0 -> (do {{\n{none};\nreturn Nothing\n}});\n1 -> (do {{\n{some});\nreturn (Just bg_v)\n}})))",
                    self.var_count,
                    operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::ResultLower {
                result,
                ty,
                results: types,
            } => {
                let ok = self.blocks.pop().unwrap().to_string();
                let err = self.blocks.pop().unwrap().to_string();
                let vars = types
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("bg_v{}", self.var_count + i))
                    .collect::<Vec<String>>();
                self.var_count += vars.len();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- case {} of {{\nLeft bg_payload -> do {{\n{err}\n}};\nRight bg_payload -> do {{\n{ok}\n}} }}\n",
                    vars.join(", "),
                    operands[0]
                ));
                results.extend(vars);
            }
            Instruction::ResultLift { result, ty } => {
                let err = self.blocks.pop().unwrap().to_string();
                let ok = self.blocks.pop().unwrap().to_string();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "bg_v{} <- (case {} of\n0 -> (do {{\n{err};\nreturn (Left bg_v)\n}});\n1 -> (do {{\n{ok});\nreturn (Right bg_v)\n}})))",
                    self.var_count,
                    operands[0]
                ));
                results.push(format!("bg_v{}", self.var_count));
                self.var_count += 1;
            }
            Instruction::CallWasm { name, sig } => {
                let vars = sig
                    .results
                    .iter()
                    .enumerate()
                    .map(|(i, _result)| format!("bg_v{}", self.var_count + i))
                    .collect::<Vec<String>>();
                self.blocks.last_mut().unwrap().push_str(&format!(
                    "({}) <- ({} {});\n",
                    vars.join(", "),
                    self.dual_func,
                    operands.join(" ")
                ));
                results.extend(vars);
                self.var_count += sig.results.len();
            }
            Instruction::CallInterface { func } => {
                let vars = (0..func.results.len())
                    .map(|i| format!("bg_v{}", self.var_count + i))
                    .collect::<Vec<String>>();
                self.var_count += vars.len();
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
            Instruction::GuestDeallocateString => todo!(),
            Instruction::GuestDeallocateList { element } => todo!(),
            Instruction::GuestDeallocateVariant { blocks } => todo!(),
        }
    }

    fn return_pointer(&mut self, size: usize, align: usize) -> Self::Operand {
        self.blocks.last_mut().unwrap().push_str(&format!(
            "bg_ret_ptr <- (callocBytes :: Int -> IO (Ptr Word8)) {size};\n"
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
            .push_str(&format!("bg_v <- return ({})", operand.join(", ")));
    }

    fn sizes(&self) -> &SizeAlign {
        &self.size_align
    }

    fn is_list_canonical(&self, _resolve: &Resolve, element: &Type) -> bool {
        match element {
            Type::Bool
            | Type::U8
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
            Type::String | Type::Id(_) => false,
        }
    }
}

fn bitcast(op: &String, cast: &Bitcast) -> String {
    match cast {
        Bitcast::F32ToI32 => format!("(castFloatToWord32 {op})"),
        Bitcast::F64ToI64 => format!("(castDoubleToWord64 {op})"),
        Bitcast::I32ToI64 => format!("((fromIntegral :: Word32 -> Word64) {op})"),
        Bitcast::F32ToI64 => {
            format!("((fromIntegral :: Word32 -> Word64) (castFloatToWord32) {op})")
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

fn gen_func(resolve: &Resolve, func: &Function, ns: Option<&str>) -> String {
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
            if name.contains(" ") && !name.starts_with("(") && !name.starts_with("[") {
                name = format!("({name})");
            }
            src.push_str(&name);
        }
    }
    let mut size_align = SizeAlign::new(AddressSize::Wasm32);
    size_align.fill(resolve);
    let mut bindgen = HsFunc {
        ns: None,
        dual_func: func_name_foreign(func, ns, Direction::Import),
        params: func
            .params
            .iter()
            .map(|(name, _ty)| lower_ident(None, &name))
            .collect(),
        blocks: vec![Source::default()],
        var_count: 0,
        size_align,
        variant: AbiVariant::GuestImport,
    };
    src.push('\n');
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

fn gen_func_core(
    resolve: &Resolve,
    func: &Function,
    ns: Option<&str>,
    variant: AbiVariant,
) -> String {
    let mut src = String::new();
    let sig = resolve.wasm_signature(variant, func);
    src.push_str(&format!(
        "foreign {} ccall \"{}\" {} :: ",
        if variant == AbiVariant::GuestExport {
            "export"
        } else {
            "import"
        },
        func.core_export_name(ns),
        func_name_foreign(
            func,
            ns,
            if variant == AbiVariant::GuestExport {
                Direction::Export
            } else {
                Direction::Import
            }
        ),
    ));
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
    if variant == AbiVariant::GuestExport {
        let mut size_align = SizeAlign::new(AddressSize::Wasm32);
        size_align.fill(resolve);
        let mut bindgen = HsFunc {
            ns: None,
            dual_func: func_name(func, ns),
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
        src.push_str(&format!(
            "{} {} = ",
            func_name_foreign(func, ns, Direction::Export),
            bindgen.params.join(" ")
        ));
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

fn func_name_foreign(func: &Function, ns: Option<&str>, dir: Direction) -> String {
    format!(
        "bg_fn_{}_{}",
        if dir == Direction::Import {
            "imp"
        } else {
            "exp"
        },
        lower_ident(ns, &func.core_export_name(None))
            .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    )
}

fn func_name(func: &Function, ns: Option<&str>) -> String {
    if let Some(ns) = ns {
        format!("Exports.{}", lower_ident(Some(ns), &func.name))
    } else {
        lower_ident(None, &func.name)
    }
}

fn lower_ident(ns: Option<&str>, name: &str) -> String {
    format!(
        "{}{}",
        if let Some(ns) = ns {
            format!("{}.", upper_ident(None, &ns))
        } else {
            "".to_owned()
        },
        name.to_lower_camel_case()
    )
}

fn upper_ident(ns: Option<&str>, name: &str) -> String {
    format!(
        "{}{}",
        if let Some(ns) = ns {
            format!("{}.", ns.to_upper_camel_case())
        } else {
            "".to_owned()
        },
        name.to_upper_camel_case()
    )
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
            let ns = ns.map(|n| format!("Types.{}", upper_ident(None, &n)));
            if let Some(name) = &ty.name {
                if let Some(ns) = ns {
                    format!("{ns}.{}", upper_ident(None, name))
                } else {
                    upper_ident(ns.as_deref(), name)
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
