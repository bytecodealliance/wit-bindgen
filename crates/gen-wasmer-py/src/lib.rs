use heck::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::mem;
use wit_bindgen_gen_core::wit_parser::abi::{
    AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType, WitxInstruction,
};
use wit_bindgen_gen_core::{wit_parser::*, Direction, Files, Generator, Ns};

#[derive(Default)]
pub struct WasmtimePy {
    src: Source,
    in_import: bool,
    opts: Opts,
    guest_imports: HashMap<String, Imports>,
    guest_exports: HashMap<String, Exports>,
    sizes: SizeAlign,
    needs_clamp: bool,
    needs_store: bool,
    needs_load: bool,
    needs_validate_guest_char: bool,
    needs_expected: bool,
    needs_i32_to_f32: bool,
    needs_f32_to_i32: bool,
    needs_i64_to_f64: bool,
    needs_f64_to_i64: bool,
    needs_decode_utf8: bool,
    needs_encode_utf8: bool,
    needs_list_canon_lift: bool,
    needs_list_canon_lower: bool,
    needs_push_buffer: bool,
    needs_pull_buffer: bool,
    needs_t_typevar: bool,
    pyimports: BTreeMap<String, Option<BTreeSet<String>>>,
}

#[derive(Default)]
struct Imports {
    freestanding_funcs: Vec<Import>,
    resource_funcs: BTreeMap<ResourceId, Vec<Import>>,
}

struct Import {
    name: String,
    src: Source,
    wasm_ty: String,
    pysig: String,
}

#[derive(Default)]
struct Exports {
    freestanding_funcs: Vec<Source>,
    resource_funcs: BTreeMap<ResourceId, Vec<Source>>,
    fields: BTreeMap<String, &'static str>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    #[cfg_attr(feature = "structopt", structopt(long = "no-typescript"))]
    pub no_typescript: bool,
}

impl Opts {
    pub fn build(self) -> WasmtimePy {
        let mut r = WasmtimePy::new();
        r.opts = self;
        r
    }
}

impl WasmtimePy {
    pub fn new() -> WasmtimePy {
        WasmtimePy::default()
    }

    fn abi_variant(dir: Direction) -> AbiVariant {
        // This generator uses a reversed mapping! In the Wasmer-py host-side
        // bindings, we don't use any extra adapter layer between guest wasm
        // modules and the host. When the guest imports functions using the
        // `GuestImport` ABI, the host directly implements the `GuestImport`
        // ABI, even though the host is *exporting* functions. Similarly, when
        // the guest exports functions using the `GuestExport` ABI, the host
        // directly imports them with the `GuestExport` ABI, even though the
        // host is *importing* functions.
        match dir {
            Direction::Import => AbiVariant::GuestExport,
            Direction::Export => AbiVariant::GuestImport,
        }
    }

    fn indent(&mut self) {
        self.src.indent(2);
    }

    fn deindent(&mut self) {
        self.src.deindent(2);
    }

    fn print_intrinsics(&mut self, iface: &Interface) {
        if self.needs_clamp {
            self.src.push_str(
                "
                    def _clamp(i: int, min: int, max: int) -> int:
                        if i < min or i > max:
                            raise OverflowError(f'must be between {min} and {max}')
                        return i
                ",
            );
        }
        if self.needs_store {
            self.pyimport("typing", "Callable");
            // TODO: this uses native endianness
            self.src.push_str(
                "
                    def _store(make_view: Callable[[], Any], mem: wasmer.Memory, base: int, offset: int, val: Any) -> None:
                        ptr = (base & 0xffffffff) + offset
                        view = make_view()
                        if ptr + view.bytes_per_element > mem.data_size:
                            raise IndexError('out-of-bounds store')
                        view_ptr = ptr // view.bytes_per_element
                        view[view_ptr] = val
                ",
            );
        }
        if self.needs_load {
            self.pyimport("typing", "Callable");
            // TODO: this uses native endianness
            self.src.push_str(
                "
                    def _load(make_view: Callable[[], Any], mem: wasmer.Memory, base: int, offset: int) -> Any:
                        ptr = (base & 0xffffffff) + offset
                        view = make_view()
                        if ptr + view.bytes_per_element > mem.data_size:
                            raise IndexError('out-of-bounds load')
                        view_ptr = ptr // view.bytes_per_element
                        return view[view_ptr]
                ",
            );
        }
        if self.needs_validate_guest_char {
            self.src.push_str(
                "
                    def _validate_guest_char(i: int) -> str:
                        if i > 0x10ffff or (i >= 0xd800 and i <= 0xdfff):
                            raise TypeError('not a valid char');
                        return chr(i)
                ",
            );
        }
        if self.needs_expected {
            self.pyimport("dataclasses", "dataclass");
            self.pyimport("typing", "TypeVar");
            self.pyimport("typing", "Generic");
            self.pyimport("typing", "Union");
            self.needs_t_typevar = true;
            self.src.push_str(
                "
                    @dataclass
                    class Ok(Generic[T]):
                        value: T
                    E = TypeVar('E')
                    @dataclass
                    class Err(Generic[E]):
                        value: E

                    Expected = Union[Ok[T], Err[E]]
                ",
            );
        }
        if self.needs_i32_to_f32 || self.needs_f32_to_i32 {
            self.pyimport("ctypes", None);
            self.src
                .push_str("_i32_to_f32_i32 = ctypes.pointer(ctypes.c_int32(0))\n");
            self.src.push_str(
                "_i32_to_f32_f32 = ctypes.cast(_i32_to_f32_i32, ctypes.POINTER(ctypes.c_float))\n",
            );
            if self.needs_i32_to_f32 {
                self.src.push_str(
                    "
                        def _i32_to_f32(i: int) -> float:
                            _i32_to_f32_i32[0] = i     # type: ignore
                            return _i32_to_f32_f32[0]  # type: ignore
                    ",
                );
            }
            if self.needs_f32_to_i32 {
                self.src.push_str(
                    "
                        def _f32_to_i32(i: float) -> int:
                            _i32_to_f32_f32[0] = i    # type: ignore
                            return _i32_to_f32_i32[0] # type: ignore
                    ",
                );
            }
        }
        if self.needs_i64_to_f64 || self.needs_f64_to_i64 {
            self.pyimport("ctypes", None);
            self.src
                .push_str("_i64_to_f64_i64 = ctypes.pointer(ctypes.c_int64(0))\n");
            self.src.push_str(
                "_i64_to_f64_f64 = ctypes.cast(_i64_to_f64_i64, ctypes.POINTER(ctypes.c_double))\n",
            );
            if self.needs_i64_to_f64 {
                self.src.push_str(
                    "
                        def _i64_to_f64(i: int) -> float:
                            _i64_to_f64_i64[0] = i    # type: ignore
                            return _i64_to_f64_f64[0] # type: ignore
                    ",
                );
            }
            if self.needs_f64_to_i64 {
                self.src.push_str(
                    "
                        def _f64_to_i64(i: float) -> int:
                            _i64_to_f64_f64[0] = i    # type: ignore
                            return _i64_to_f64_i64[0] # type: ignore
                    ",
                );
            }
        }
        if self.needs_decode_utf8 {
            self.src.push_str(
                "
                    def _decode_utf8(mem: wasmer.Memory, ptr: int, len: int) -> str:
                        print('ptr = ' + str(ptr))
                        print('len = ' + str(len))
                        ptr = ptr & 0xffffffff
                        len = len & 0xffffffff
                        if ptr + len > mem.data_size:
                            raise IndexError('string out of bounds')
                        view = mem.uint8_view()
                        bytes = bytearray(view[ptr:ptr+len])
                        x = bytes.decode('utf8')
                        print('decode_utf8: ' + x)
                        print('bytes: ' + str(bytes))
                        return x
                ",
            );
        }
        if self.needs_encode_utf8 {
            self.pyimport("typing", "Tuple");
            self.src.push_str(
                "
                    def _encode_utf8(val: str, realloc: wasmer.Function, mem: wasmer.Memory) -> Tuple[int, int]:
                        bytes = val.encode('utf8')
                        print('encode_utf8: ' + val)
                        print('bytes: ' + str(bytes))
                        ptr = realloc(0, 0, 1, len(bytes))
                        assert(isinstance(ptr, int))
                        ptr = ptr & 0xffffffff
                        if ptr + len(bytes) > mem.data_size:
                            raise IndexError('string out of bounds')
                        view = mem.uint8_view()
                        view[ptr:ptr+len(bytes)] = bytes
                        return (ptr, len(bytes))
                ",
            );
        }
        if self.needs_list_canon_lift {
            self.pyimport("typing", "List");
            self.pyimport("typing", "Callable");
            // TODO: this is doing a native-endian read, not a little-endian
            // read
            self.src.push_str(
                "
                    def _list_canon_lift(ptr: int, len: int, size: int, make_view: Callable[[], Any], mem: wasmer.Memory) -> Any:
                        ptr = ptr & 0xffffffff
                        len = len & 0xffffffff
                        if ptr + len * size > mem.data_size:
                            raise IndexError('list out of bounds')
                        view = make_view()
                        assert(size == view.bytes_per_element)
                        view_ptr = ptr // view.bytes_per_element
                        return view[view_ptr:view_ptr + len]
                ",
            );
        }
        if self.needs_list_canon_lower {
            self.pyimport("typing", "List");
            self.pyimport("typing", "Tuple");
            self.pyimport("typing", "Callable");
            // TODO: this is doing a native-endian write, not a little-endian
            // write
            self.src.push_str(
                "
                    def _list_canon_lower(list: Any, make_view: Callable[[], Any], size: int, align: int, realloc: wasmer.Function, mem: wasmer.Memory) -> Tuple[int, int]:
                        total_size = size * len(list)
                        ptr = realloc(0, 0, align, total_size)
                        assert(isinstance(ptr, int))
                        ptr = ptr & 0xffffffff
                        if ptr + total_size > mem.data_size:
                            raise IndexError('list realloc return of bounds')
                        view = make_view()
                        assert(size == view.bytes_per_element)
                        view_ptr = ptr // view.bytes_per_element
                        view[view_ptr:view_ptr + len(list)] = list
                        return (ptr, len(list))
                ",
            );
        }

        if iface.resources.len() > 0 {
            self.pyimport("typing", "TypeVar");
            self.pyimport("typing", "Generic");
            self.pyimport("typing", "List");
            self.pyimport("typing", "Optional");
            self.pyimport("dataclasses", "dataclass");
            self.needs_t_typevar = true;
            self.src.push_str(
                "
                    @dataclass
                    class SlabEntry(Generic[T]):
                        next: int
                        val: Optional[T]

                    class Slab(Generic[T]):
                        head: int
                        list: List[SlabEntry[T]]

                        def __init__(self) -> None:
                            self.list = []
                            self.head = 0

                        def insert(self, val: T) -> int:
                            if self.head >= len(self.list):
                                self.list.append(SlabEntry(next = len(self.list) + 1, val = None))
                            ret = self.head
                            slot = self.list[ret]
                            self.head = slot.next
                            slot.next = -1
                            slot.val = val
                            return ret

                        def get(self, idx: int) -> T:
                            if idx >= len(self.list):
                                raise IndexError('handle index not valid')
                            slot = self.list[idx]
                            if slot.next == -1:
                                assert(slot.val is not None)
                                return slot.val
                            raise IndexError('handle index not valid')

                        def remove(self, idx: int) -> T:
                            ret = self.get(idx)
                            slot = self.list[idx]
                            slot.val = None
                            slot.next = self.head
                            self.head = idx
                            return ret
                ",
            );
        }
        if self.needs_push_buffer {
            self.pyimport("typing", "TypeVar");
            self.pyimport("typing", "Generic");
            self.pyimport("typing", "Callable");
            self.needs_t_typevar = true;
            self.src.push_str(
                "
                    class PushBuffer(Generic[T]):
                        def __init__(self, ptr: int, len: int, size: int, write: Callable) -> None:
                            self.ptr = ptr
                            self.len = len
                            self.size = size
                            self.write = write

                        def __len__(self) -> int:
                            return self.len

                        def push(self, val: T) -> bool:
                            if self.len == 0:
                                return False;
                            self.len -= 1;
                            self.write(val, self.ptr);
                            self.ptr += self.size;
                            return True
                ",
            )
        }
        if self.needs_pull_buffer {
            self.pyimport("typing", "TypeVar");
            self.pyimport("typing", "Generic");
            self.pyimport("typing", "Callable");
            self.pyimport("typing", "Optional");
            self.needs_t_typevar = true;
            self.src.push_str(
                "
                    class PullBuffer(Generic[T]):
                        def __init__(self, ptr: int, len: int, size: int, read: Callable) -> None:
                            self.len = len
                            self.ptr = ptr
                            self.size = size
                            self.read = read

                        def __len__(self) -> int:
                            return self.len

                        def pull(self) -> Optional[T]:
                            if self.len == 0:
                                return None
                            self.len -= 1
                            ret: T = self.read(self.ptr)
                            self.ptr += self.size
                            return ret
                ",
            )
        }
    }

    fn type_string(&mut self, iface: &Interface, ty: &Type) -> String {
        let prev = mem::take(&mut self.src);
        self.print_ty(iface, ty);
        mem::replace(&mut self.src, prev).into()
    }

    fn print_ty(&mut self, iface: &Interface, ty: &Type) {
        match ty {
            Type::U8
            | Type::S8
            | Type::U16
            | Type::S16
            | Type::U32
            | Type::S32
            | Type::U64
            | Type::S64
            | Type::Usize
            | Type::CChar => self.src.push_str("int"),
            Type::F32 | Type::F64 => self.src.push_str("float"),
            Type::Char => self.src.push_str("str"),
            Type::Handle(id) => {
                // In general we want to use quotes around this to support
                // forward-references (such as a method on a resource returning
                // that resource), but that would otherwise generate type alias
                // annotations that look like `Foo = 'HandleType'` which isn't
                // creating a type alias but rather a string definition. Hack
                // around that here to detect the type alias scenario and don't
                // surround the type with quotes, otherwise surround with
                // single-quotes.
                let suffix = if self.src.ends_with(" = ") {
                    ""
                } else {
                    self.src.push_str("'");
                    "'"
                };
                self.src
                    .push_str(&iface.resources[*id].name.to_camel_case());
                self.src.push_str(suffix);
            }
            Type::Id(id) => {
                let ty = &iface.types[*id];
                if let Some(name) = &ty.name {
                    self.src.push_str(&name.to_camel_case());
                    return;
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(iface, t),
                    TypeDefKind::Record(r) if r.is_tuple() => {
                        self.print_tuple(iface, r.fields.iter().map(|f| &f.ty))
                    }
                    TypeDefKind::Record(_) => unreachable!(),
                    TypeDefKind::Variant(v) => {
                        if v.is_bool() {
                            self.src.push_str("bool");
                        } else if let Some(t) = v.as_option() {
                            self.pyimport("typing", "Optional");
                            self.src.push_str("Optional[");
                            self.print_ty(iface, t);
                            self.src.push_str("]");
                        } else if let Some((ok, err)) = v.as_expected() {
                            self.needs_expected = true;
                            self.src.push_str("Expected[");
                            match ok {
                                Some(t) => self.print_ty(iface, t),
                                None => self.src.push_str("None"),
                            }
                            self.src.push_str(", ");
                            match err {
                                Some(t) => self.print_ty(iface, t),
                                None => self.src.push_str("None"),
                            }
                            self.src.push_str("]");
                        } else {
                            unreachable!()
                        }
                    }
                    TypeDefKind::List(t) => self.print_list(iface, t),
                    TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => {
                        self.src.push_str("int")
                    }
                    TypeDefKind::PushBuffer(t) => self.print_buffer(iface, true, t),
                    TypeDefKind::PullBuffer(t) => self.print_buffer(iface, false, t),
                }
            }
        }
    }

    fn print_tuple<'a>(&mut self, iface: &Interface, types: impl IntoIterator<Item = &'a Type>) {
        let types = types.into_iter().collect::<Vec<_>>();
        if types.is_empty() {
            return self.src.push_str("None");
        }
        self.pyimport("typing", "Tuple");
        self.src.push_str("Tuple[");
        for (i, t) in types.iter().enumerate() {
            if i > 0 {
                self.src.push_str(", ");
            }
            self.print_ty(iface, t);
        }
        self.src.push_str("]");
    }

    fn print_buffer(&mut self, iface: &Interface, push: bool, ty: &Type) {
        if push {
            self.needs_push_buffer = true;
            self.src.push_str("PushBuffer");
        } else {
            self.needs_pull_buffer = true;
            self.src.push_str("PullBuffer");
        }
        self.src.push_str("[");
        self.print_ty(iface, ty);
        self.src.push_str("]");
    }

    fn print_list(&mut self, iface: &Interface, element: &Type) {
        match element {
            Type::Char => self.src.push_str("str"),
            Type::U8 => self.src.push_str("bytes"),
            t => {
                self.pyimport("typing", "List");
                self.src.push_str("List[");
                self.print_ty(iface, t);
                self.src.push_str("]");
            }
        }
    }

    fn pyimport<'a>(&mut self, module: &str, name: impl Into<Option<&'a str>>) {
        let name = name.into();
        let list = self
            .pyimports
            .entry(module.to_string())
            .or_insert(match name {
                Some(_) => Some(BTreeSet::new()),
                None => None,
            });
        match name {
            Some(name) => {
                assert!(list.is_some());
                list.as_mut().unwrap().insert(name.to_string());
            }
            None => assert!(list.is_none()),
        }
    }

    fn array_ty(&self, iface: &Interface, ty: &Type) -> Option<&'static str> {
        match ty {
            Type::U8 | Type::CChar => Some("uint8"),
            Type::S8 => Some("int8"),
            Type::U16 => Some("uint16"),
            Type::S16 => Some("int16"),
            Type::U32 | Type::Usize => Some("uint32"),
            Type::S32 => Some("int32"),
            Type::U64 => Some("uint64"),
            Type::S64 => Some("int64"),
            Type::F32 => Some("float32"),
            Type::F64 => Some("float64"),
            Type::Char => None,
            Type::Handle(_) => None,
            Type::Id(id) => match &iface.types[*id].kind {
                TypeDefKind::Type(t) => self.array_ty(iface, t),
                _ => None,
            },
        }
    }

    fn docs(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        for line in docs.lines() {
            self.src.push_str(&format!("# {}\n", line));
        }
    }

    fn print_sig(&mut self, iface: &Interface, func: &Function) -> Vec<String> {
        if !self.in_import {
            if let FunctionKind::Static { .. } = func.kind {
                self.src.push_str("@classmethod\n");
            }
        }
        self.src.push_str("def ");
        match &func.kind {
            FunctionKind::Method { .. } => self.src.push_str(&func.item_name().to_snake_case()),
            FunctionKind::Static { .. } if !self.in_import => {
                self.src.push_str(&func.item_name().to_snake_case())
            }
            _ => self.src.push_str(&func.name.to_snake_case()),
        }
        if self.in_import {
            self.src.push_str("(self");
        } else if let FunctionKind::Static { .. } = func.kind {
            self.src.push_str("(cls, obj: '");
            self.src.push_str(&iface.name.to_camel_case());
            self.src.push_str("'");
        } else {
            self.src.push_str("(self");
        }
        let mut params = Vec::new();
        for (i, (param, ty)) in func.params.iter().enumerate() {
            if i == 0 {
                if let FunctionKind::Method { .. } = func.kind {
                    params.push("self".to_string());
                    continue;
                }
            }
            self.src.push_str(", ");
            self.src.push_str(&param.to_snake_case());
            params.push(param.to_snake_case());
            self.src.push_str(": ");
            self.print_ty(iface, ty);
        }
        self.src.push_str(") -> ");
        match func.results.len() {
            0 => self.src.push_str("None"),
            1 => self.print_ty(iface, &func.results[0].1),
            _ => self.print_tuple(iface, func.results.iter().map(|p| &p.1)),
        }
        params
    }
}

impl Generator for WasmtimePy {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        let variant = Self::abi_variant(dir);
        self.sizes.fill(variant, iface);
        self.in_import = variant == AbiVariant::GuestImport;
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        self.docs(docs);
        if record.is_tuple() {
            self.src.push_str(&format!("{} = ", name.to_camel_case()));
            self.print_tuple(iface, record.fields.iter().map(|f| &f.ty));
        } else if record.is_flags() {
            self.pyimport("enum", "Flag");
            self.pyimport("enum", "auto");
            self.src
                .push_str(&format!("class {}(Flag):\n", name.to_camel_case()));
            self.indent();
            for field in record.fields.iter() {
                self.docs(&field.docs);
                self.src
                    .push_str(&format!("{} = auto()\n", field.name.to_shouty_snake_case()));
            }
            if record.fields.is_empty() {
                self.src.push_str("pass\n");
            }
            self.deindent();
        } else {
            self.pyimport("dataclasses", "dataclass");
            self.src
                .push_str(&format!("@dataclass\nclass {}:\n", name.to_camel_case()));
            self.indent();
            for field in record.fields.iter() {
                self.docs(&field.docs);
                self.src
                    .push_str(&format!("{}: ", field.name.to_snake_case()));
                self.print_ty(iface, &field.ty);
                self.src.push_str("\n");
            }
            if record.fields.is_empty() {
                self.src.push_str("pass\n");
            }
            self.deindent();
        }
        self.src.push_str("\n");
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        self.docs(docs);
        if variant.is_bool() {
            self.src
                .push_str(&format!("{} = bool\n", name.to_camel_case()));
        } else if variant.is_enum() {
            self.pyimport("enum", "Enum");
            self.src
                .push_str(&format!("class {}(Enum):\n", name.to_camel_case()));
            self.indent();
            for (i, case) in variant.cases.iter().enumerate() {
                self.docs(&case.docs);

                // TODO this handling of digits should be more general and
                // shouldn't be here just to fix the one case in wasi where an
                // enum variant is "2big" and doesn't generate valid Python. We
                // should probably apply this to all generated Python
                // identifiers.
                let mut name = case.name.to_shouty_snake_case();
                if name.chars().next().unwrap().is_digit(10) {
                    name = format!("_{}", name);
                }
                self.src.push_str(&format!("{} = {}\n", name, i));
            }
            self.deindent();
        } else if let Some(t) = variant.as_option() {
            self.pyimport("typing", "Optional");
            self.src
                .push_str(&format!("{} = Optional[", name.to_camel_case()));
            self.print_ty(iface, t);
            self.src.push_str("]\n");
        } else if let Some((ok, err)) = variant.as_expected() {
            self.needs_expected = true;
            self.src
                .push_str(&format!("{} = Expected[", name.to_camel_case()));
            match ok {
                Some(t) => self.print_ty(iface, t),
                None => self.src.push_str("None"),
            }
            self.src.push_str(", ");
            match err {
                Some(t) => self.print_ty(iface, t),
                None => self.src.push_str("None"),
            }
            self.src.push_str("]\n");
        } else {
            self.pyimport("dataclasses", "dataclass");
            let mut cases = Vec::new();
            for case in variant.cases.iter() {
                self.docs(&case.docs);
                self.src.push_str("@dataclass\n");
                let name = format!("{}{}", name.to_camel_case(), case.name.to_camel_case());
                self.src.push_str(&format!("class {}:\n", name));
                self.indent();
                match &case.ty {
                    Some(ty) => {
                        self.src.push_str("value: ");
                        self.print_ty(iface, ty);
                        self.src.push_str("\n");
                    }
                    None => self.src.push_str("pass\n"),
                }
                self.deindent();
                self.src.push_str("\n");
                cases.push(name);
            }

            self.pyimport("typing", "Union");
            self.src.push_str(&format!(
                "{} = Union[{}]\n",
                name.to_camel_case(),
                cases.join(", "),
            ));
        }
        self.src.push_str("\n");
    }

    fn type_resource(&mut self, _iface: &Interface, _ty: ResourceId) {
        // if !self.in_import {
        //     self.exported_resources.insert(ty);
        // }
    }

    fn type_alias(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.docs(docs);
        self.src.push_str(&format!("{} = ", name.to_camel_case()));
        self.print_ty(iface, ty);
        self.src.push_str("\n");
    }

    fn type_list(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.docs(docs);
        self.src.push_str(&format!("{} = ", name.to_camel_case()));
        self.print_list(iface, ty);
        self.src.push_str("\n");
    }

    fn type_pointer(
        &mut self,
        _iface: &Interface,
        _id: TypeId,
        _name: &str,
        _const_: bool,
        _ty: &Type,
        _docs: &Docs,
    ) {
        // drop((iface, _id, name, const_, ty, docs));
    }

    fn type_builtin(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.type_alias(iface, id, name, ty, docs);
    }

    fn type_push_buffer(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
        self.docs(docs);
        self.src.push_str(&format!("{} = ", name.to_camel_case()));
        self.print_buffer(iface, true, ty);
        self.src.push_str("\n");
    }

    fn type_pull_buffer(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
        self.docs(docs);
        self.src.push_str(&format!("{} = ", name.to_camel_case()));
        self.print_buffer(iface, false, ty);
        self.src.push_str("\n");
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "export" uses the "guest import" ABI variant on the inside of
    // this `Generator` implementation.
    fn export(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");
        let prev = mem::take(&mut self.src);

        self.print_sig(iface, func);
        let pysig = mem::take(&mut self.src).into();

        let sig = iface.wasm_signature(AbiVariant::GuestImport, func);
        self.src
            .push_str(&format!("def {}(", func.name.to_snake_case(),));
        let mut params = Vec::new();
        for (i, param) in sig.params.iter().enumerate() {
            if i != 0 {
                self.src.push_str(", ");
            }
            let name = format!("arg{}", i);
            self.src.push_str(&name);
            self.src.push_str(": ");
            self.src.push_str(wasm_ty_typing(*param));
            params.push(name);
        }
        self.src.push_str(") -> ");
        match sig.results.len() {
            0 => self.src.push_str("None"),
            1 => self.src.push_str(wasm_ty_typing(sig.results[0])),
            _ => unimplemented!(),
        }
        self.src.push_str(":\n");
        self.indent();

        let mut f = FunctionBindgen::new(self, params);
        iface.call(
            AbiVariant::GuestImport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );

        let FunctionBindgen {
            src,
            needs_memory,
            needs_realloc,
            needs_free,
            mut locals,
            ..
        } = f;

        if needs_memory {
            // TODO: hardcoding "memory"
            self.src.push_str("m = get_export(\"memory\")\n");
            self.src.push_str("assert(isinstance(m, wasmer.Memory))\n");
            self.pyimport("typing", "cast");
            self.src.push_str("memory = cast(wasmer.Memory, m)\n");
            locals.insert("memory").unwrap();
        }

        if let Some(name) = needs_realloc {
            self.src
                .push_str(&format!("realloc = get_export(\"{}\")\n", name));
            self.src
                .push_str("assert(isinstance(realloc, wasmer.Function))\n");
            locals.insert("realloc").unwrap();
        }

        if let Some(name) = needs_free {
            self.src.push_str(&format!("free = get_export(\"{}\")\n", name));
            self.src
                .push_str("assert(isinstance(free, wasmer.Function))\n");
            locals.insert("free").unwrap();
        }
        self.src.push_str(&src);
        self.deindent();

        let src = mem::replace(&mut self.src, prev);
        let mut wasm_ty = String::from("wasmer.FunctionType([");
        wasm_ty.push_str(
            &sig.params
                .iter()
                .map(|t| wasm_ty_ctor(*t))
                .collect::<Vec<_>>()
                .join(", "),
        );
        wasm_ty.push_str("], [");
        wasm_ty.push_str(
            &sig.results
                .iter()
                .map(|t| wasm_ty_ctor(*t))
                .collect::<Vec<_>>()
                .join(", "),
        );
        wasm_ty.push_str("])");
        let import = Import {
            name: func.name.clone(),
            src,
            wasm_ty,
            pysig,
        };
        let imports = self
            .guest_imports
            .entry(iface.name.to_string())
            .or_insert(Imports::default());
        let dst = match &func.kind {
            FunctionKind::Freestanding | FunctionKind::Static { .. } => {
                &mut imports.freestanding_funcs
            }
            FunctionKind::Method { resource, .. } => imports
                .resource_funcs
                .entry(*resource)
                .or_insert(Vec::new()),
        };
        dst.push(import);
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "import" uses the "export" ABI variant on the inside of
    // this `Generator` implementation.
    fn import(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");
        let prev = mem::take(&mut self.src);

        let params = self.print_sig(iface, func);
        self.src.push_str(":\n");
        self.indent();

        let src_object = match &func.kind {
            FunctionKind::Freestanding => "self".to_string(),
            FunctionKind::Static { .. } => "obj".to_string(),
            FunctionKind::Method { .. } => "self._obj".to_string(),
        };
        let mut f = FunctionBindgen::new(self, params);
        f.src_object = src_object;
        iface.call(
            AbiVariant::GuestExport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );

        let FunctionBindgen {
            src,
            needs_memory,
            needs_realloc,
            needs_free,
            src_object,
            ..
        } = f;
        if needs_memory {
            // TODO: hardcoding "memory"
            self.src
                .push_str(&format!("memory = {}._memory;\n", src_object));
        }

        if let Some(name) = &needs_realloc {
            self.src.push_str(&format!(
                "realloc = {}._{}\n",
                src_object,
                name.to_snake_case(),
            ));
        }

        if let Some(name) = &needs_free {
            self.src.push_str(&format!(
                "free = {}._{}\n",
                src_object,
                name.to_snake_case(),
            ));
        }
        self.src.push_str(&src);
        self.deindent();

        let exports = self
            .guest_exports
            .entry(iface.name.to_string())
            .or_insert_with(Exports::default);
        if needs_memory {
            exports.fields.insert("memory".to_string(), "wasmer.Memory");
        }
        if let Some(name) = &needs_realloc {
            exports.fields.insert(name.clone(), "wasmer.Function");
        }
        if let Some(name) = &needs_free {
            exports.fields.insert(name.clone(), "wasmer.Function");
        }
        exports.fields.insert(func.name.clone(), "wasmer.Function");

        let func_body = mem::replace(&mut self.src, prev);
        let dst = match &func.kind {
            FunctionKind::Freestanding => &mut exports.freestanding_funcs,
            FunctionKind::Static { resource, .. } | FunctionKind::Method { resource, .. } => {
                exports
                    .resource_funcs
                    .entry(*resource)
                    .or_insert(Vec::new())
            }
        };
        dst.push(func_body);
    }

    fn finish_one(&mut self, iface: &Interface, files: &mut Files) {
        self.pyimport("typing", "Any");
        self.pyimport("abc", "abstractmethod");
        self.pyimport("typing", "Callable");

        let types = mem::take(&mut self.src);
        self.print_intrinsics(iface);
        let intrinsics = mem::take(&mut self.src);

        for (k, v) in self.pyimports.iter() {
            match v {
                Some(list) => {
                    let list = list.iter().cloned().collect::<Vec<_>>().join(", ");
                    self.src.push_str(&format!("from {} import {}\n", k, list));
                }
                None => {
                    self.src.push_str(&format!("import {}\n", k));
                }
            }
        }
        self.src.push_str("import wasmer # type: ignore\n");
        self.src.push_str(
            "
                try:
                    from typing import Protocol
                except ImportError:
                    class Protocol: # type: ignore
                        pass
            ",
        );
        self.src.push_str("\n");

        if self.needs_t_typevar {
            self.src.push_str("T = TypeVar('T')\n");
        }

        self.src.push_str(&intrinsics);
        for (id, r) in iface.resources.iter() {
            let name = r.name.to_camel_case();
            if self.in_import {
                self.src.push_str(&format!("class {}(Protocol):\n", name));
                self.src.indent(2);
                self.src.push_str("def drop(self) -> None:\n");
                self.src.indent(2);
                self.src.push_str("pass\n");
                self.src.deindent(2);

                for (_, funcs) in self.guest_imports.iter() {
                    if let Some(funcs) = funcs.resource_funcs.get(&id) {
                        for func in funcs {
                            self.src.push_str("@abstractmethod\n");
                            self.src.push_str(&func.pysig);
                            self.src.push_str(":\n");
                            self.src.indent(2);
                            self.src.push_str("raise NotImplementedError\n");
                            self.src.deindent(2);
                        }
                    }
                }
                self.src.deindent(2);
            } else {
                self.src.push_str(&format!("class {}:\n", name));
                self.src.indent(2);
                self.src.push_str(&format!(
                    "
                        _wasm_val: int
                        _refcnt: int
                        _obj: '{iface}'
                        _destroyed: bool

                        def __init__(self, val: int, obj: '{iface}') -> None:
                            self._wasm_val = val
                            self._refcnt = 1
                            self._obj = obj
                            self._destroyed = False

                        def clone(self) -> '{name}':
                            self._refcnt += 1
                            return self

                        def drop(self) -> None:
                            self._refcnt -= 1;
                            if self._refcnt != 0:
                                return
                            assert(not self._destroyed)
                            self._destroyed = True
                            self._obj._canonical_abi_drop_{drop}(self._wasm_val)

                        def __del__(self) -> None:
                            if not self._destroyed:
                                raise RuntimeError('wasm object not dropped')
                    ",
                    name = name,
                    iface = iface.name.to_camel_case(),
                    drop = name.to_snake_case(),
                ));

                for (_, exports) in self.guest_exports.iter() {
                    if let Some(funcs) = exports.resource_funcs.get(&id) {
                        for func in funcs {
                            self.src.push_str(func);
                        }
                    }
                }

                self.src.deindent(2);
            }
        }
        self.src.push_str(&types);

        for (module, funcs) in mem::take(&mut self.guest_imports) {
            self.src
                .push_str(&format!("class {}(Protocol):\n", module.to_camel_case()));
            self.indent();
            for func in funcs.freestanding_funcs.iter() {
                self.src.push_str("@abstractmethod\n");
                self.src.push_str(&func.pysig);
                self.src.push_str(":\n");
                self.indent();
                self.src.push_str("raise NotImplementedError\n");
                self.deindent();
            }
            self.deindent();
            self.src.push_str("\n");

            self.src.push_str(&format!(
                "def add_{}_to_imports(store: wasmer.Store, imports: dict[str, dict[str, Any]], host: {}, get_export: Callable[[str], Any]) -> None:\n",
                module.to_snake_case(),
                module.to_camel_case(),
            ));
            self.indent();

            for (id, r) in iface.resources.iter() {
                self.src.push_str(&format!(
                    "_resources{}: Slab[{}] = Slab()\n",
                    id.index(),
                    r.name.to_camel_case()
                ));
            }

            for func in funcs
                .freestanding_funcs
                .iter()
                .chain(funcs.resource_funcs.values().flat_map(|v| v))
            {
                self.src.push_str(&format!("ty = {}\n", func.wasm_ty));
                self.src.push_str(&func.src);
                self.src.push_str(&format!(
                    "imports.setdefault('{}', {{}})['{}'] = wasmer.Function(store, {}, ty)\n",
                    iface.name,
                    func.name,
                    func.name.to_snake_case(),
                ));
            }

            for (id, resource) in iface.resources.iter() {
                let snake = resource.name.to_snake_case();

                self.src.push_str(&format!(
                    "def resource_drop_{}(i: int) -> None:\n  _resources{}.remove(i).drop()\n",
                    snake,
                    id.index(),
                ));
                self.src
                    .push_str("ty = wasmer.FunctionType([wasmer.Type.I32], [])\n");
                self.src.push_str(&format!(
                    "imports.setdefault('canonical_abi', {{}})['resource_drop_{}'] = \
                        wasmer.Function(store, resource_drop_{}, ty)\n",
                    resource.name, snake,
                ));
            }
            self.deindent();
        }

        // This is exculsively here to get mypy to not complain about empty
        // modules, this probably won't really get triggered much in practice
        if !self.in_import && self.guest_exports.is_empty() {
            self.src
                .push_str(&format!("class {}:\n", iface.name.to_camel_case()));
            self.indent();
            if iface.resources.len() == 0 {
                self.src.push_str("pass\n");
            } else {
                for (_, r) in iface.resources.iter() {
                    self.src.push_str(&format!(
                        "_canonical_abi_drop_{}: wasmer.Function\n",
                        r.name.to_snake_case(),
                    ));
                }
            }
            self.deindent();
        }

        for (module, exports) in mem::take(&mut self.guest_exports) {
            let module = module.to_camel_case();
            self.src.push_str(&format!("class {}:\n", module));
            self.indent();

            self.src.push_str("instance: wasmer.Instance\n");
            for (name, ty) in exports.fields.iter() {
                self.src
                    .push_str(&format!("_{}: {}\n", name.to_snake_case(), ty));
            }
            for (id, r) in iface.resources.iter() {
                self.src.push_str(&format!(
                    "_resource{}_slab: Slab[{}]\n",
                    id.index(),
                    r.name.to_camel_case(),
                ));
                self.src.push_str(&format!(
                    "_canonical_abi_drop_{}: wasmer.Function\n",
                    r.name.to_snake_case(),
                ));
            }

            self.src.push_str("def __init__(self, store: wasmer.Store, imports: dict[str, dict[str, Any]], module: wasmer.Module):\n");
            self.indent();
            for (id, r) in iface.resources.iter() {
                self.src.push_str(&format!(
                    "
                       ty1 = wasmer.FunctionType([wasmer.Type.I32], [])
                       ty2 = wasmer.FunctionType([wasmer.Type.I32], [wasmer.Type.I32])
                       def drop_{snake}(idx: int) -> None:
                            self._resource{idx}_slab.remove(idx).drop();
                       imports.setdefault('canonical_abi', {{}})['resource_drop_{name}'] = wasmer.Function(store, drop_{snake}, ty1)

                       def clone_{snake}(idx: int) -> int:
                            obj = self._resource{idx}_slab.get(idx)
                            return self._resource{idx}_slab.insert(obj.clone())
                       imports.setdefault('canonical_abi', {{}})['resource_clone_{name}'] = wasmer.Function(store, clone_{snake}, ty2)

                       def get_{snake}(idx: int) -> int:
                            obj = self._resource{idx}_slab.get(idx)
                            return obj._wasm_val
                       imports.setdefault('canonical_abi', {{}})['resource_get_{name}'] = wasmer.Function(store, get_{snake}, ty2)

                       def new_{snake}(val: int) -> int:
                            return self._resource{idx}_slab.insert({camel}(val, self))
                       imports.setdefault('canonical_abi', {{}})['resource_new_{name}'] = wasmer.Function(store, new_{snake}, ty2)
                   ",
                    name = r.name,
                    camel = r.name.to_camel_case(),
                    snake = r.name.to_snake_case(),
                    idx = id.index(),
                ));
            }
            self.src
                .push_str("self.instance = wasmer.Instance(module, imports)\n");
            for (name, ty) in exports.fields.iter() {
                self.src.push_str(&format!(
                    "
                        {snake} = self.instance.exports.__getattribute__('{name}')
                        assert(isinstance({snake}, {ty}))
                        self._{snake} = {snake}
                    ",
                    name = name,
                    snake = name.to_snake_case(),
                    ty = ty,
                ));
            }
            for (id, r) in iface.resources.iter() {
                self.src.push_str(&format!(
                    "
                        self._resource{idx}_slab = Slab()
                        canon_drop_{snake} = self.instance.exports.__getattribute__('canonical_abi_drop_{name}')
                        assert(isinstance(canon_drop_{snake}, wasmer.Function))
                        self._canonical_abi_drop_{snake} = canon_drop_{snake}
                    ",
                    idx = id.index(),
                    name = r.name,
                    snake = r.name.to_snake_case(),
                ));
            }
            self.deindent();

            for func in exports.freestanding_funcs.iter() {
                self.src.push_str(&func);
            }

            self.deindent();
        }

        files.push("bindings.py", self.src.as_bytes());
    }
}

struct FunctionBindgen<'a> {
    gen: &'a mut WasmtimePy,
    locals: Ns,
    src: Source,
    block_storage: Vec<Source>,
    blocks: Vec<(String, Vec<String>)>,
    needs_memory: bool,
    needs_realloc: Option<String>,
    needs_free: Option<String>,
    params: Vec<String>,
    payloads: Vec<String>,
    src_object: String,
}

impl FunctionBindgen<'_> {
    fn new(gen: &mut WasmtimePy, params: Vec<String>) -> FunctionBindgen<'_> {
        let mut locals = Ns::default();
        locals.insert("len").unwrap(); // python built-in
        locals.insert("base").unwrap(); // may be used as loop var
        locals.insert("i").unwrap(); // may be used as loop var
        for param in params.iter() {
            locals.insert(param).unwrap();
        }
        FunctionBindgen {
            gen,
            locals,
            src: Source::default(),
            block_storage: Vec::new(),
            blocks: Vec::new(),
            needs_memory: false,
            needs_realloc: None,
            needs_free: None,
            params,
            payloads: Vec::new(),
            src_object: "self".to_string(),
        }
    }

    fn clamp<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        self.gen.needs_clamp = true;
        results.push(format!("_clamp({}, {}, {})", operands[0], min, max));
    }

    fn load(&mut self, ty: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        self.needs_memory = true;
        self.gen.needs_load = true;
        let tmp = self.locals.tmp("load");
        self.src.push_str(&format!(
            "{} = _load(memory.{}_view, memory, {}, {})\n",
            tmp, ty, operands[0], offset,
        ));
        results.push(tmp);
    }

    fn store(&mut self, ty: &str, offset: i32, operands: &[String]) {
        self.needs_memory = true;
        self.gen.needs_store = true;
        self.src.push_str(&format!(
            "_store(memory.{}_view, memory, {}, {}, {})\n",
            ty, operands[1], offset, operands[0]
        ));
    }
}

impl Bindgen for FunctionBindgen<'_> {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
    }

    fn push_block(&mut self) {
        let prev = mem::take(&mut self.src);
        self.block_storage.push(prev);
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src, to_restore);
        self.blocks.push((src.into(), mem::take(operands)));
    }

    fn allocate_typed_space(&mut self, _iface: &Interface, _ty: TypeId) -> String {
        unimplemented!()
    }

    fn i64_return_pointer_area(&mut self, _amt: usize) -> String {
        unimplemented!()
    }

    fn is_list_canonical(&self, iface: &Interface, ty: &Type) -> bool {
        self.gen.array_ty(iface, ty).is_some()
    }

    fn emit(
        &mut self,
        iface: &Interface,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(val.to_string()),
            Instruction::ConstZero { tys } => {
                for t in tys.iter() {
                    match t {
                        WasmType::I32 | WasmType::I64 => results.push("0".to_string()),
                        WasmType::F32 | WasmType::F64 => results.push("0.0".to_string()),
                    }
                }
            }

            // The representation of i32 in Python is a number, so 8/16-bit
            // values get further clamped to ensure that the upper bits aren't
            // set when we pass the value, ensuring that only the right number
            // of bits are transferred.
            Instruction::U8FromI32 => self.clamp(results, operands, u8::MIN, u8::MAX),
            Instruction::S8FromI32 => self.clamp(results, operands, i8::MIN, i8::MAX),
            Instruction::U16FromI32 => self.clamp(results, operands, u16::MIN, u16::MAX),
            Instruction::S16FromI32 => self.clamp(results, operands, i16::MIN, i16::MAX),
            // Ensure the bits of the number are treated as unsigned.
            Instruction::U32FromI32 | Instruction::UsizeFromI32 => {
                results.push(format!("{} & 0xffffffff", operands[0]));
            }
            // All bigints coming from wasm are treated as signed, so convert
            // it to ensure it's treated as unsigned.
            Instruction::U64FromI64 => {
                results.push(format!("{} & 0xffffffffffffffff", operands[0]));
            }
            // Nothing to do signed->signed where the representations are the
            // same.
            Instruction::S32FromI32 | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap())
            }

            // All values coming from the host and going to wasm need to have
            // their ranges validated, since the host could give us any value.
            Instruction::I32FromU8 => self.clamp(results, operands, u8::MIN, u8::MAX),
            Instruction::I32FromS8 => self.clamp(results, operands, i8::MIN, i8::MAX),
            Instruction::I32FromU16 => self.clamp(results, operands, u16::MIN, u16::MAX),
            Instruction::I32FromS16 => self.clamp(results, operands, i16::MIN, i16::MAX),
            // TODO: need to do something to get this to be represented as signed?
            Instruction::I32FromU32 | Instruction::I32FromUsize => {
                self.clamp(results, operands, u32::MIN, u32::MAX);
            }
            Instruction::I32FromS32 => self.clamp(results, operands, i32::MIN, i32::MAX),
            // TODO: need to do something to get this to be represented as signed?
            Instruction::I64FromU64 => self.clamp(results, operands, u64::MIN, u64::MAX),
            Instruction::I64FromS64 => self.clamp(results, operands, i64::MIN, i64::MAX),

            // Python uses `float` for f32/f64, so everything is equivalent
            // here.
            Instruction::If32FromF32
            | Instruction::If64FromF64
            | Instruction::F32FromIf32
            | Instruction::F64FromIf64 => results.push(operands.pop().unwrap()),

            // Validate that i32 values coming from wasm are indeed valid code
            // points.
            Instruction::CharFromI32 => {
                self.gen.needs_validate_guest_char = true;
                results.push(format!("_validate_guest_char({})", operands[0]));
            }

            Instruction::I32FromChar => {
                results.push(format!("ord({})", operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                for (cast, op) in casts.iter().zip(operands) {
                    match cast {
                        Bitcast::I32ToF32 => {
                            self.gen.needs_i32_to_f32 = true;
                            results.push(format!("_i32_to_f32({})", op));
                        }
                        Bitcast::F32ToI32 => {
                            self.gen.needs_f32_to_i32 = true;
                            results.push(format!("_f32_to_i32({})", op));
                        }
                        Bitcast::F32ToF64 | Bitcast::F64ToF32 => results.push(op.clone()),
                        Bitcast::I64ToF64 => {
                            self.gen.needs_i64_to_f64 = true;
                            results.push(format!("_i64_to_f64({})", op));
                        }
                        Bitcast::F64ToI64 => {
                            self.gen.needs_f64_to_i64 = true;
                            results.push(format!("_f64_to_i64({})", op));
                        }
                        Bitcast::I64ToF32 => {
                            self.gen.needs_i32_to_f32 = true;
                            results.push(format!("_i32_to_f32(({}) & 0xffffffff)", op));
                        }
                        Bitcast::F32ToI64 => {
                            self.gen.needs_f32_to_i32 = true;
                            results.push(format!("_f32_to_i32({})", op));
                        }
                        Bitcast::I32ToI64 | Bitcast::I64ToI32 | Bitcast::None => {
                            results.push(op.clone())
                        }
                    }
                }
            }

            // These instructions are used with handles when we're implementing
            // imports. This means we interact with the `resources` slabs to
            // translate the wasm-provided index into a Python value.
            Instruction::I32FromOwnedHandle { ty } => {
                results.push(format!("_resources{}.insert({})", ty.index(), operands[0]));
            }
            Instruction::HandleBorrowedFromI32 { ty } => {
                results.push(format!("_resources{}.get({})", ty.index(), operands[0]));
            }

            // These instructions are used for handles to objects owned in wasm.
            // This means that they're interacting with a wrapper class defined
            // in Python.
            Instruction::I32FromBorrowedHandle { ty } => {
                let obj = self.locals.tmp("obj");
                self.src.push_str(&format!("{} = {}\n", obj, operands[0]));

                results.push(format!(
                    "{}._resource{}_slab.insert({}.clone())",
                    self.src_object,
                    ty.index(),
                    obj,
                ));
            }
            Instruction::HandleOwnedFromI32 { ty } => {
                results.push(format!(
                    "{}._resource{}_slab.remove({})",
                    self.src_object,
                    ty.index(),
                    operands[0],
                ));
            }
            Instruction::RecordLower { record, .. } => {
                if record.fields.is_empty() {
                    return;
                }
                if record.is_tuple() {
                    self.src.push_str("(");
                    for _ in 0..record.fields.len() {
                        let name = self.locals.tmp("tuplei");
                        self.src.push_str(&name);
                        self.src.push_str(",");
                        results.push(name);
                    }
                    self.src.push_str(") = ");
                    self.src.push_str(&operands[0]);
                    self.src.push_str("\n");
                } else {
                    let tmp = self.locals.tmp("record");
                    self.src.push_str(&format!("{} = {}\n", tmp, operands[0]));
                    for field in record.fields.iter() {
                        let name = self.locals.tmp("field");
                        self.src.push_str(&format!(
                            "{} = {}.{}\n",
                            name,
                            tmp,
                            field.name.to_snake_case(),
                        ));
                        results.push(name);
                    }
                }
            }

            Instruction::RecordLift { record, name, .. } => {
                if record.is_tuple() {
                    if operands.is_empty() {
                        results.push("None".to_string());
                    } else {
                        results.push(format!("({},)", operands.join(", ")));
                    }
                } else {
                    let name = name.unwrap();
                    results.push(format!("{}({})", name.to_camel_case(), operands.join(", ")));
                }
            }
            Instruction::FlagsLift { name, .. } | Instruction::FlagsLift64 { name, .. } => {
                results.push(format!("{}({})", name.to_camel_case(), operands[0]));
            }
            Instruction::FlagsLower { .. } | Instruction::FlagsLower64 { .. } => {
                results.push(format!("({}).value", operands[0]));
            }

            Instruction::VariantPayloadName => {
                let name = self.locals.tmp("payload");
                results.push(name.clone());
                self.payloads.push(name);
            }
            Instruction::BufferPayloadName => results.push("e".to_string()),
            Instruction::VariantLower {
                variant,
                results: result_types,
                name,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let payloads = self
                    .payloads
                    .drain(self.payloads.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                if result_types.len() == 1 && variant.is_enum() {
                    if variant.is_bool() {
                        return results.push(format!("int({})", operands[0]));
                    }
                    if name.is_some() {
                        return results.push(format!("({}).value", operands[0]));
                    }
                }

                for _ in 0..result_types.len() {
                    results.push(self.locals.tmp("variant"));
                }

                let mut needs_else = true;
                for (i, ((case, (block, block_results)), payload)) in
                    variant.cases.iter().zip(blocks).zip(payloads).enumerate()
                {
                    if i == 0 {
                        self.src.push_str("if ");
                    } else if i == 1 && variant.as_option().is_some() {
                        needs_else = false;
                        self.src.push_str("else:\n");
                    } else {
                        self.src.push_str("elif ");
                    }

                    if variant.is_bool() {
                        self.src
                            .push_str(&format!("{} == bool({}):\n", operands[0], i));
                    } else if variant.as_option().is_some() {
                        if i == 0 {
                            self.src.push_str(&format!("{} is None:\n", operands[0]));
                        }
                    } else if variant.is_enum() && variant.as_expected().is_none() {
                        self.src
                            .push_str(&format!("{}.value == {}:\n", operands[0], i));
                    } else {
                        self.src.push_str(&format!(
                            "isinstance({}, {}{}):\n",
                            operands[0],
                            if variant.as_expected().is_some() {
                                String::new()
                            } else {
                                name.unwrap().to_camel_case()
                            },
                            case.name.to_camel_case()
                        ));
                    }
                    self.src.indent(2);

                    if case.ty.is_some() {
                        if variant.as_option().is_some() {
                            self.src
                                .push_str(&format!("{} = {}\n", payload, operands[0]));
                        } else {
                            self.src
                                .push_str(&format!("{} = {}.value\n", payload, operands[0]));
                        }
                    }

                    self.src.push_str(&block);

                    for (i, result) in block_results.iter().enumerate() {
                        self.src.push_str(&format!("{} = {}\n", results[i], result));
                    }
                    self.src.deindent(2);
                }
                if needs_else {
                    let variant_name = name.map(|s| s.to_camel_case());
                    let variant_name = variant_name.as_deref().unwrap_or_else(|| {
                        if variant.is_bool() {
                            "bool"
                        } else if variant.as_expected().is_some() {
                            "expected"
                        } else if variant.as_option().is_some() {
                            "option"
                        } else {
                            unimplemented!()
                        }
                    });
                    self.src.push_str("else:\n");
                    self.src.indent(2);
                    self.src.push_str(&format!(
                        "raise TypeError(\"invalid variant specified for {}\")\n",
                        variant_name
                    ));
                    self.src.deindent(2);
                }
            }

            Instruction::VariantLift {
                variant, name, ty, ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                if let Some(name) = name {
                    if variant.is_enum() {
                        results.push(format!("{}({})", name.to_camel_case(), operands[0]));
                        return;
                    }
                }

                let result = self.locals.tmp("variant");
                self.src.push_str(&format!(
                    "{}: {}\n",
                    result,
                    self.gen.type_string(iface, &Type::Id(*ty)),
                ));
                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    if i == 0 {
                        self.src.push_str("if ");
                    } else {
                        self.src.push_str("elif ");
                    }
                    self.src.push_str(&format!("{} == {}:\n", operands[0], i));
                    self.src.indent(2);
                    self.src.push_str(&block);

                    if variant.is_bool() {
                        assert!(block_results.is_empty());
                        self.src
                            .push_str(&format!("{} = {}\n", result, case.name.to_camel_case(),));
                    } else if variant.as_option().is_some() {
                        if case.ty.is_none() {
                            assert!(block_results.is_empty());
                            self.src.push_str(&format!("{} = None\n", result));
                        } else {
                            assert!(block_results.len() == 1);
                            self.src
                                .push_str(&format!("{} = {}\n", result, block_results[0]));
                        }
                    } else {
                        self.src.push_str(&format!(
                            "{} = {}{}(",
                            result,
                            if variant.as_expected().is_some() {
                                String::new()
                            } else {
                                name.unwrap().to_camel_case()
                            },
                            case.name.to_camel_case()
                        ));
                        if case.ty.is_some() {
                            assert!(block_results.len() == 1);
                            self.src.push_str(&block_results[0]);
                        } else {
                            assert!(block_results.is_empty());
                            if variant.as_expected().is_some() {
                                self.src.push_str("None");
                            }
                        }
                        self.src.push_str(")\n");
                    }
                    self.src.deindent(2);
                }
                self.src.push_str("else:\n");
                self.src.indent(2);
                let variant_name = name.map(|s| s.to_camel_case());
                let variant_name = variant_name.as_deref().unwrap_or_else(|| {
                    if variant.is_bool() {
                        "bool"
                    } else if variant.as_expected().is_some() {
                        "expected"
                    } else if variant.as_option().is_some() {
                        "option"
                    } else {
                        unimplemented!()
                    }
                });
                self.src.push_str(&format!(
                    "raise TypeError(\"invalid variant discriminant for {}\")\n",
                    variant_name
                ));
                self.src.deindent(2);
                results.push(result);
            }

            Instruction::ListCanonLower { element, realloc } => {
                // Lowering only happens when we're passing lists into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                let realloc = realloc.unwrap();
                self.needs_memory = true;
                self.needs_realloc = Some(realloc.to_string());

                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                match element {
                    Type::Char => {
                        self.gen.needs_encode_utf8 = true;
                        self.src.push_str(&format!(
                            "{}, {} = _encode_utf8({}, realloc, memory)\n",
                            ptr, len, operands[0],
                        ));
                    }
                    _ => {
                        let array_ty = self.gen.array_ty(iface, element).unwrap();
                        self.gen.needs_list_canon_lower = true;
                        let size = self.gen.sizes.size(element);
                        let align = self.gen.sizes.align(element);
                        self.src.push_str(&format!(
                            "{}, {} = _list_canon_lower({}, memory.{}_view, {}, {}, realloc, memory)\n",
                            ptr, len, operands[0], array_ty, size, align,
                        ));
                    }
                };
                results.push(ptr);
                results.push(len);
            }
            Instruction::ListCanonLift { element, free, .. } => {
                self.needs_memory = true;
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                self.src.push_str(&format!("{} = {}\n", ptr, operands[0]));
                self.src.push_str(&format!("{} = {}\n", len, operands[1]));
                let (result, align) = match element {
                    Type::Char => {
                        self.gen.needs_decode_utf8 = true;
                        (format!("_decode_utf8(memory, {}, {})", ptr, len), 1)
                    }
                    _ => {
                        let array_ty = self.gen.array_ty(iface, element).unwrap();
                        self.gen.needs_list_canon_lift = true;
                        let mut lift = format!(
                            "_list_canon_lift({}, {}, {}, memory.{}_view, memory)",
                            ptr,
                            len,
                            self.gen.sizes.size(element),
                            array_ty,
                        );
                        let pyty = match element {
                            Type::U8 => {
                                lift = format!("bytes({})", lift);
                                "bytes".to_string()
                            },
                            _ => {
                                self.gen.pyimport("typing", "List");
                                format!("List[{}]", self.gen.type_string(iface, element))
                            }
                        };
                        self.gen.pyimport("typing", "cast");
                        (
                            format!("cast({}, {})", pyty, lift),
                            self.gen.sizes.align(element),
                        )
                    }
                };
                match free {
                    Some(free) => {
                        self.needs_free = Some(free.to_string());
                        let list = self.locals.tmp("list");
                        self.src.push_str(&format!("{} = {}\n", list, result));
                        self.src
                            .push_str(&format!("free({}, {}, {})\n", ptr, len, align));
                        results.push(list);
                    }
                    None => results.push(result),
                }
            }

            Instruction::ListLower { element, realloc } => {
                let base = self.payloads.pop().unwrap();
                let e = self.payloads.pop().unwrap();
                let realloc = realloc.unwrap();
                let (body, body_results) = self.blocks.pop().unwrap();
                assert!(body_results.is_empty());
                let vec = self.locals.tmp("vec");
                let result = self.locals.tmp("result");
                let len = self.locals.tmp("len");
                self.needs_realloc = Some(realloc.to_string());
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                self.src.push_str(&format!("{} = {}\n", vec, operands[0]));
                self.src.push_str(&format!("{} = len({})\n", len, vec));

                // ... then realloc space for the result in the guest module
                self.src.push_str(&format!(
                    "{} = realloc(0, 0, {}, {} * {})\n",
                    result, align, len, size,
                ));
                self.src
                    .push_str(&format!("assert(isinstance({}, int))\n", result));

                // ... then consume the vector and use the block to lower the
                // result.
                let i = self.locals.tmp("i");
                self.src
                    .push_str(&format!("for {} in range(0, {}):\n", i, len));
                self.src.indent(2);
                self.src.push_str(&format!("{} = {}[{}]\n", e, vec, i));
                self.src
                    .push_str(&format!("{} = {} + {} * {}\n", base, result, i, size));
                self.src.push_str(&body);
                self.src.deindent(2);

                results.push(result);
                results.push(len);
            }

            Instruction::ListLift { element, free, .. } => {
                let (body, body_results) = self.blocks.pop().unwrap();
                let base = self.payloads.pop().unwrap();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                self.src.push_str(&format!("{} = {}\n", ptr, operands[0]));
                self.src.push_str(&format!("{} = {}\n", len, operands[1]));
                let result = self.locals.tmp("result");
                let ty = self.gen.type_string(iface, element);
                self.src
                    .push_str(&format!("{}: List[{}] = []\n", result, ty));

                let i = self.locals.tmp("i");
                self.src
                    .push_str(&format!("for {} in range(0, {}):\n", i, len));
                self.src.indent(2);
                self.src
                    .push_str(&format!("{} = {} + {} * {}\n", base, ptr, i, size));
                self.src.push_str(&body);
                assert_eq!(body_results.len(), 1);
                self.src
                    .push_str(&format!("{}.append({})\n", result, body_results[0]));
                self.src.deindent(2);

                if let Some(free) = free {
                    self.needs_free = Some(free.to_string());
                    self.src
                        .push_str(&format!("free({}, {} * {}, {})\n", ptr, len, size, align,));
                }
                results.push(result);
            }

            Instruction::IterElem { .. } => {
                let name = self.locals.tmp("e");
                results.push(name.clone());
                self.payloads.push(name);
            }
            Instruction::IterBasePointer => {
                let name = self.locals.tmp("base");
                results.push(name.clone());
                self.payloads.push(name);
            }

            Instruction::BufferLiftPtrLen { push, ty } => {
                let (block, block_results) = self.blocks.pop().unwrap();
                let base = self.payloads.pop().unwrap();
                self.needs_memory = true;
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                self.src.push_str(&format!("{} = {}\n", ptr, operands[1]));
                self.src.push_str(&format!("{} = {}\n", len, operands[2]));
                let size = self.gen.sizes.size(ty);
                if *push {
                    self.gen.needs_push_buffer = true;
                    assert!(block_results.is_empty());
                    let write = self.locals.tmp("write_val");
                    self.src
                        .push_str(&format!("def {}(e, {}): # type: ignore\n", write, base));
                    self.src.indent(2);
                    self.src.push_str(&block);
                    self.src.deindent(2);
                    results.push(format!("PushBuffer({}, {}, {}, {})", ptr, len, size, write));
                } else {
                    self.gen.needs_pull_buffer = true;
                    assert_eq!(block_results.len(), 1);
                    let read = self.locals.tmp("read_val");
                    self.src
                        .push_str(&format!("def {}({}): # type: ignore\n", read, base));
                    self.src.indent(2);
                    self.src.push_str(&block);
                    self.src.push_str(&format!("return {}\n", block_results[0]));
                    self.src.deindent(2);
                    results.push(format!("PullBuffer({}, {}, {}, {})", ptr, len, size, read));
                }
            }

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
            Instruction::CallWasm {
                module: _,
                name,
                sig,
            } => {
                if sig.results.len() > 0 {
                    for i in 0..sig.results.len() {
                        if i > 0 {
                            self.src.push_str(", ");
                        }
                        let ret = self.locals.tmp("ret");
                        self.src.push_str(&ret);
                        results.push(ret);
                    }
                    self.src.push_str(" = ");
                }
                self.src.push_str(&self.src_object);
                self.src.push_str("._");
                self.src.push_str(&name.to_snake_case());
                self.src.push_str("(");
                self.src.push_str(&operands.join(", "));
                self.src.push_str(")\n");
                for (ty, name) in sig.results.iter().zip(results.iter()) {
                    let ty = match ty {
                        WasmType::I32 | WasmType::I64 => "int",
                        WasmType::F32 | WasmType::F64 => "float",
                    };
                    self.src
                        .push_str(&format!("assert(isinstance({}, {}))\n", name, ty));
                }
            }
            Instruction::CallInterface { module: _, func } => {
                for i in 0..func.results.len() {
                    if i > 0 {
                        self.src.push_str(", ");
                    }
                    let result = self.locals.tmp("ret");
                    self.src.push_str(&result);
                    results.push(result);
                }
                if func.results.len() > 0 {
                    self.src.push_str(" = ");
                }
                match &func.kind {
                    FunctionKind::Freestanding | FunctionKind::Static { .. } => {
                        self.src.push_str(&format!(
                            "host.{}({})",
                            func.name.to_snake_case(),
                            operands.join(", "),
                        ));
                    }
                    FunctionKind::Method { name, .. } => {
                        self.src.push_str(&format!(
                            "{}.{}({})",
                            operands[0],
                            name.to_snake_case(),
                            operands[1..].join(", "),
                        ));
                    }
                }
                self.src.push_str("\n");
            }

            Instruction::Return { amt, .. } => match amt {
                0 => {}
                1 => self.src.push_str(&format!("return {}\n", operands[0])),
                _ => {
                    self.src
                        .push_str(&format!("return ({})\n", operands.join(", ")));
                }
            },

            Instruction::I32Load { offset } => self.load("int32", *offset, operands, results),
            Instruction::I64Load { offset } => self.load("int64", *offset, operands, results),
            Instruction::F32Load { offset } => self.load("float32", *offset, operands, results),
            Instruction::F64Load { offset } => self.load("float64", *offset, operands, results),
            Instruction::I32Load8U { offset } => self.load("uint8", *offset, operands, results),
            Instruction::I32Load8S { offset } => self.load("int8", *offset, operands, results),
            Instruction::I32Load16U { offset } => self.load("uint16", *offset, operands, results),
            Instruction::I32Load16S { offset } => self.load("int16", *offset, operands, results),
            Instruction::I32Store { offset } => self.store("uint32", *offset, operands),
            Instruction::I64Store { offset } => self.store("uint64", *offset, operands),
            Instruction::F32Store { offset } => self.store("float32", *offset, operands),
            Instruction::F64Store { offset } => self.store("float64", *offset, operands),
            Instruction::I32Store8 { offset } => self.store("uint8", *offset, operands),
            Instruction::I32Store16 { offset } => self.store("uint16", *offset, operands),

            Instruction::Witx { instr } => match instr {
                WitxInstruction::PointerFromI32 { .. } => results.push(operands[0].clone()),
                i => unimplemented!("{:?}", i),
            },
            i => unimplemented!("{:?}", i),
        }
    }
}

#[derive(Default)]
pub struct Source {
    s: String,
    indent: usize,
}

impl Source {
    pub fn push_str(&mut self, src: &str) {
        let lines = src.lines().collect::<Vec<_>>();
        let mut trim = None;
        for (i, line) in lines.iter().enumerate() {
            self.s.push_str(if lines.len() == 1 {
                line
            } else {
                let trim = match trim {
                    Some(n) => n,
                    None => {
                        let val = line.len() - line.trim_start().len();
                        if !line.is_empty() {
                            trim = Some(val);
                        }
                        val
                    }
                };
                line.get(trim..).unwrap_or("")
            });
            if i != lines.len() - 1 || src.ends_with("\n") {
                self.newline();
            }
        }
    }

    pub fn indent(&mut self, amt: usize) {
        self.indent += amt;
        for _ in 0..amt {
            self.s.push_str("  ");
        }
    }

    pub fn deindent(&mut self, amt: usize) {
        self.indent -= amt;
        for _ in 0..amt {
            assert!(self.s.ends_with("  "));
            self.s.pop();
            self.s.pop();
        }
    }

    fn newline(&mut self) {
        self.s.push_str("\n");
        for _ in 0..self.indent {
            self.s.push_str("  ");
        }
    }
}

impl std::ops::Deref for Source {
    type Target = str;
    fn deref(&self) -> &str {
        &self.s
    }
}

impl From<Source> for String {
    fn from(s: Source) -> String {
        s.s
    }
}

fn wasm_ty_ctor(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "wasmer.Type.I32",
        WasmType::I64 => "wasmer.Type.I64",
        WasmType::F32 => "wasmer.Type.F32",
        WasmType::F64 => "wasmer.Type.F64",
    }
}

fn wasm_ty_typing(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "int",
        WasmType::I64 => "int",
        WasmType::F32 => "float",
        WasmType::F64 => "float",
    }
}

#[cfg(test)]
mod tests {
    use super::Source;

    #[test]
    fn simple_append() {
        let mut s = Source::default();
        s.push_str("x");
        assert_eq!(s.s, "x");
        s.push_str("y");
        assert_eq!(s.s, "xy");
        s.push_str("z ");
        assert_eq!(s.s, "xyz ");
        s.push_str(" a ");
        assert_eq!(s.s, "xyz  a ");
        s.push_str("\na");
        assert_eq!(s.s, "xyz  a \na");
    }

    #[test]
    fn trim_ws() {
        let mut s = Source::default();
        s.push_str("def foo():\n  return 1\n");
        assert_eq!(s.s, "def foo():\n  return 1\n");
    }
}
