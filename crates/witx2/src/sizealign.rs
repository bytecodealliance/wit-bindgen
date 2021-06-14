use crate::abi::CallMode;
use crate::{Int, Interface, Record, RecordKind, Type, TypeDef, TypeDefKind, Variant};

#[derive(Default)]
pub struct SizeAlign {
    map: Vec<(usize, usize)>,
}

impl SizeAlign {
    pub fn fill(&mut self, mode: CallMode, iface: &Interface) {
        self.map = vec![(0, 0); iface.types.len()];
        for ty in iface.topological_types() {
            let pair = self.calculate(mode, &iface.types[ty]);
            self.map[ty.index()] = pair;
        }
    }

    fn calculate(&self, mode: CallMode, ty: &TypeDef) -> (usize, usize) {
        match &ty.kind {
            TypeDefKind::Type(t) => (self.size(t), self.align(t)),
            TypeDefKind::List(_) => (8, 4),
            TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => (4, 4),
            TypeDefKind::PushBuffer(_) | TypeDefKind::PullBuffer(_) => {
                if mode.import() {
                    (12, 4)
                } else {
                    (4, 4)
                }
            }
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
                let mut size = 0;
                let mut align = 1;
                for f in r.fields.iter() {
                    let field_size = self.size(&f.ty);
                    let field_align = self.align(&f.ty);
                    size = align_to(size, field_align) + field_size;
                    align = align.max(field_align);
                }
                (align_to(size, align), align)
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
            Type::U8 | Type::S8 | Type::CChar => 1,
            Type::U16 | Type::S16 => 2,
            Type::U32 | Type::S32 | Type::F32 | Type::Char | Type::Handle(_) | Type::Usize => 4,
            Type::U64 | Type::S64 | Type::F64 => 8,
            Type::Id(id) => self.map[id.index()].0,
        }
    }

    pub fn align(&self, ty: &Type) -> usize {
        match ty {
            Type::U8 | Type::S8 | Type::CChar => 1,
            Type::U16 | Type::S16 => 2,
            Type::U32 | Type::S32 | Type::F32 | Type::Char | Type::Handle(_) | Type::Usize => 4,
            Type::U64 | Type::S64 | Type::F64 => 8,
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
}

fn int_size_align(i: Int) -> (usize, usize) {
    match i {
        Int::U8 => (1, 1),
        Int::U16 => (2, 2),
        Int::U32 => (4, 4),
        Int::U64 => (8, 8),
    }
}

fn align_to(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}
