use anyhow::{bail, Result};
use std::collections::HashSet;
use std::fmt::Write;
use wit_parser::{Enum, Flags, Interface, Record, Tuple, Type, TypeDefKind, TypeId, Variant};

/// A utility for printing WebAssembly interface definitions to a string.
#[derive(Default)]
pub struct InterfacePrinter {
    output: String,
    declared: HashSet<TypeId>,
}

impl InterfacePrinter {
    /// Print the given WebAssembly interface to a string.
    pub fn print(&mut self, interface: &Interface) -> Result<String> {
        for func in &interface.functions {
            for ty in func.params.iter().map(|p| &p.1).chain([&func.result]) {
                self.declare_type(interface, ty)?;
            }
        }

        for func in &interface.functions {
            write!(&mut self.output, "{}: function(", func.name)?;
            for (i, (name, ty)) in func.params.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                write!(&mut self.output, "{}: ", name)?;
                self.print_type_name(interface, ty)?;
            }
            self.output.push(')');

            match &func.result {
                Type::Unit => {}
                other => {
                    self.output.push_str(" -> ");
                    self.print_type_name(interface, other)?;
                }
            }
            self.output.push_str("\n\n");
        }

        self.declared.clear();
        Ok(std::mem::take(&mut self.output))
    }

    fn print_type_name(&mut self, interface: &Interface, ty: &Type) -> Result<()> {
        match ty {
            Type::Unit => self.output.push_str("unit"),
            Type::Bool => self.output.push_str("bool"),
            Type::U8 => self.output.push_str("u8"),
            Type::U16 => self.output.push_str("u16"),
            Type::U32 => self.output.push_str("u32"),
            Type::U64 => self.output.push_str("u64"),
            Type::S8 => self.output.push_str("s8"),
            Type::S16 => self.output.push_str("s16"),
            Type::S32 => self.output.push_str("s32"),
            Type::S64 => self.output.push_str("s64"),
            Type::Float32 => self.output.push_str("float32"),
            Type::Float64 => self.output.push_str("float64"),
            Type::Char => self.output.push_str("char"),
            Type::String => self.output.push_str("string"),

            Type::Id(id) => {
                let ty = &interface.types[*id];
                if let Some(name) = &ty.name {
                    self.output.push_str(name);
                    return Ok(());
                }

                match &ty.kind {
                    TypeDefKind::Tuple(t) => {
                        self.print_tuple_type(interface, t)?;
                    }
                    TypeDefKind::Record(_) => {
                        bail!("interface has an unnamed record type");
                    }
                    TypeDefKind::Flags(_) => {
                        bail!("interface has unnamed flags type")
                    }
                    TypeDefKind::Enum(_) => {
                        bail!("interface has unnamed enum type")
                    }
                    TypeDefKind::Variant(v) => {
                        self.print_variant_type(interface, v)?;
                    }
                    TypeDefKind::List(ty) => {
                        self.output.push_str("list<");
                        self.print_type_name(interface, ty)?;
                        self.output.push('>');
                    }
                    TypeDefKind::Type(ty) => self.print_type_name(interface, ty)?,
                }
            }

            Type::Handle(_) => bail!("interface has unsupported type"),
        }

        Ok(())
    }

    fn print_tuple_type(&mut self, interface: &Interface, tuple: &Tuple) -> Result<()> {
        self.output.push_str("tuple<");
        for (i, ty) in tuple.types.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.print_type_name(interface, ty)?;
        }
        self.output.push('>');

        Ok(())
    }

    fn print_variant_type(&mut self, interface: &Interface, variant: &Variant) -> Result<()> {
        if let Some(ty) = variant.as_option() {
            self.output.push_str("option<");
            self.print_type_name(interface, ty)?;
            self.output.push('>');
            return Ok(());
        }

        if let Some((ok, err)) = variant.as_expected() {
            self.output.push_str("expected<");
            match ok {
                Some(ty) => self.print_type_name(interface, ty)?,
                None => self.output.push('_'),
            }
            self.output.push_str(", ");
            match err {
                Some(ty) => self.print_type_name(interface, ty)?,
                None => self.output.push('_'),
            }
            self.output.push('>');
            return Ok(());
        }

        bail!("interface has an unnamed variant type");
    }

    fn declare_type(&mut self, interface: &Interface, ty: &Type) -> Result<()> {
        match ty {
            Type::Unit
            | Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::S8
            | Type::S16
            | Type::S32
            | Type::S64
            | Type::Float32
            | Type::Float64
            | Type::Char
            | Type::String => return Ok(()),

            Type::Id(id) => {
                if !self.declared.insert(*id) {
                    return Ok(());
                }

                let ty = &interface.types[*id];
                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        self.declare_record(interface, ty.name.as_deref(), r)?
                    }
                    TypeDefKind::Tuple(t) => {
                        self.declare_tuple(interface, ty.name.as_deref(), t)?
                    }
                    TypeDefKind::Flags(f) => self.declare_flags(ty.name.as_deref(), f)?,
                    TypeDefKind::Variant(v) => {
                        self.declare_variant(interface, ty.name.as_deref(), v)?
                    }
                    TypeDefKind::Enum(e) => self.declare_enum(ty.name.as_deref(), e)?,
                    TypeDefKind::List(inner) => {
                        self.declare_list(interface, ty.name.as_deref(), inner)?
                    }
                    TypeDefKind::Type(inner) => match ty.name.as_deref() {
                        Some(name) => {
                            write!(&mut self.output, "type {} = ", name)?;
                            self.print_type_name(interface, inner)?;
                            self.output.push_str("\n\n");
                        }
                        None => bail!("unnamed type in interface"),
                    },
                }
            }

            Type::Handle(_) => bail!("interface has unsupported type"),
        }
        Ok(())
    }

    fn declare_record(
        &mut self,
        interface: &Interface,
        name: Option<&str>,
        record: &Record,
    ) -> Result<()> {
        for field in record.fields.iter() {
            self.declare_type(interface, &field.ty)?;
        }

        match name {
            Some(name) => {
                writeln!(&mut self.output, "record {} {{", name)?;
                for field in &record.fields {
                    write!(&mut self.output, "  {}: ", field.name)?;
                    self.declare_type(interface, &field.ty)?;
                    self.print_type_name(interface, &field.ty)?;
                    self.output.push_str(",\n");
                }
                self.output.push_str("}\n\n");
                Ok(())
            }
            None => bail!("interface has unnamed record type"),
        }
    }

    fn declare_tuple(
        &mut self,
        interface: &Interface,
        name: Option<&str>,
        tuple: &Tuple,
    ) -> Result<()> {
        for ty in tuple.types.iter() {
            self.declare_type(interface, ty)?;
        }

        if let Some(name) = name {
            write!(&mut self.output, "type {} = ", name)?;
            self.print_tuple_type(interface, tuple)?;
            self.output.push_str("\n\n");
        }
        return Ok(());
    }

    fn declare_flags(&mut self, name: Option<&str>, flags: &Flags) -> Result<()> {
        match name {
            Some(name) => {
                writeln!(&mut self.output, "flags {} {{", name)?;
                for flag in &flags.flags {
                    writeln!(&mut self.output, "  {},", flag.name)?;
                }
                self.output.push_str("}\n\n");
            }
            None => bail!("interface has unnamed flags type"),
        }
        Ok(())
    }

    fn declare_variant(
        &mut self,
        interface: &Interface,
        name: Option<&str>,
        variant: &Variant,
    ) -> Result<()> {
        for case in variant.cases.iter() {
            if let Some(ty) = &case.ty {
                self.declare_type(interface, ty)?;
            }
        }

        if variant.as_option().is_some() || variant.as_expected().is_some() {
            if let Some(name) = name {
                write!(&mut self.output, "type {} = ", name)?;
                self.print_variant_type(interface, variant)?;
                self.output.push_str("\n\n");
            }
            return Ok(());
        }

        match name {
            Some(name) => {
                if variant.is_union() {
                    writeln!(&mut self.output, "union {} {{", name)?;
                    for case in &variant.cases {
                        self.output.push_str("  ");
                        self.print_type_name(interface, case.ty.as_ref().unwrap())?;
                        self.output.push_str(",\n");
                    }
                    self.output.push_str("}\n\n");
                    return Ok(());
                }

                writeln!(&mut self.output, "variant {} {{", name)?;
                for case in &variant.cases {
                    write!(&mut self.output, "  {}", case.name)?;
                    if let Some(ty) = &case.ty {
                        self.output.push('(');
                        self.print_type_name(interface, ty)?;
                        self.output.push(')');
                    }
                    self.output.push_str(",\n");
                }
                self.output.push_str("}\n\n");
                Ok(())
            }
            None => bail!("interface has unnamed variant type"),
        }
    }

    fn declare_enum(&mut self, name: Option<&str>, enum_: &Enum) -> Result<()> {
        let name = match name {
            Some(name) => name,
            None => bail!("interface has unnamed enum type"),
        };
        writeln!(&mut self.output, "enum {} {{", name)?;
        for case in &enum_.cases {
            writeln!(&mut self.output, "  {},", case.name)?;
        }
        self.output.push_str("}\n\n");
        Ok(())
    }

    fn declare_list(&mut self, interface: &Interface, name: Option<&str>, ty: &Type) -> Result<()> {
        self.declare_type(interface, ty)?;

        if let Some(name) = name {
            write!(&mut self.output, "type {} = list<", name)?;
            self.print_type_name(interface, ty)?;
            self.output.push_str(">\n\n");
            return Ok(());
        }

        Ok(())
    }
}
