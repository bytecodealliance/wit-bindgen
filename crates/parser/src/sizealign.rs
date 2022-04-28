use crate::{Int, Interface, Record, RecordKind, Type, TypeDef, TypeDefKind, Variant};

#[derive(Default)]
pub struct SizeAlign {
    map: Vec<(usize, usize)>,
}

impl SizeAlign {
    pub fn fill(&mut self, iface: &Interface) {
        self.map = vec![(0, 0); iface.types.len()];
        for ty in iface.topological_types() {
            let pair = self.calculate(&iface.types[ty]);
            self.map[ty.index()] = pair;
        }
    }

    fn calculate(&self, ty: &TypeDef) -> (usize, usize) {
        match &ty.kind {
            TypeDefKind::Type(t) => (self.size(t), self.align(t)),
            TypeDefKind::List(_) => (8, 4),
            TypeDefKind::Record(r) => {
                if let RecordKind::Flags(repr) = r.kind {
                    return match repr {
                        Some(i) => int_size_align(i),
                        None if r.fields.len() <= 8 => (1, 1),
                        None if r.fields.len() <= 16 => (2, 2),
                        None if r.fields.len() <= 32 => (4, 4),
                        None if r.fields.len() <= 64 => (8, 8),
                        None => (r.num_i32s() * 4, 4),
                    };
                }
                self.record(r.fields.iter().map(|f| &f.ty))
            }
            TypeDefKind::Variant(v) => {
                let (discrim_size, discrim_align) = int_size_align(v.tag);
                let mut size = discrim_size;
                let mut align = discrim_align;
                for c in v.cases.iter() {
                    if let Some(ty) = &c.ty {
                        let case_size = self.size(ty);
                        let case_align = self.align(ty);
                        align = align.max(case_align);
                        size = size.max(align_to(discrim_size, case_align) + case_size);
                    }
                }
                (size, align)
            }
        }
    }

    pub fn size(&self, ty: &Type) -> usize {
        match ty {
            Type::Unit => 0,
            Type::Bool | Type::U8 | Type::S8 => 1,
            Type::U16 | Type::S16 => 2,
            Type::U32 | Type::S32 | Type::Float32 | Type::Char | Type::Handle(_) => 4,
            Type::U64 | Type::S64 | Type::Float64 | Type::String => 8,
            Type::Id(id) => self.map[id.index()].0,
        }
    }

    pub fn align(&self, ty: &Type) -> usize {
        match ty {
            Type::Unit | Type::Bool | Type::U8 | Type::S8 => 1,
            Type::U16 | Type::S16 => 2,
            Type::U32 | Type::S32 | Type::Float32 | Type::Char | Type::Handle(_) | Type::String => {
                4
            }
            Type::U64 | Type::S64 | Type::Float64 => 8,
            Type::Id(id) => self.map[id.index()].1,
        }
    }

    pub fn field_offsets(&self, record: &Record) -> Vec<usize> {
        let mut cur = 0;
        record
            .fields
            .iter()
            .map(|field| {
                let ret = align_to(cur, self.align(&field.ty));
                cur = ret + self.size(&field.ty);
                ret
            })
            .collect()
    }

    pub fn payload_offset(&self, variant: &Variant) -> usize {
        let mut max_align = 1;
        for c in variant.cases.iter() {
            if let Some(ty) = &c.ty {
                max_align = max_align.max(self.align(ty));
            }
        }
        let tag_size = int_size_align(variant.tag).0;
        align_to(tag_size, max_align)
    }

    pub fn record<'a>(&self, types: impl Iterator<Item = &'a Type>) -> (usize, usize) {
        let mut size = 0;
        let mut align = 1;
        for ty in types {
            let field_size = self.size(ty);
            let field_align = self.align(ty);
            size = align_to(size, field_align) + field_size;
            align = align.max(field_align);
        }
        (align_to(size, align), align)
    }
}

fn int_size_align(i: Int) -> (usize, usize) {
    match i {
        Int::U8 => (1, 1),
        Int::U16 => (2, 2),
        Int::U32 => (4, 4),
        Int::U64 => (8, 8),
    }
}

pub(crate) fn align_to(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}
