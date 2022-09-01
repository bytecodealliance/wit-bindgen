wit_bindgen_guest_rust::import!("../../tests/runtime/unions/imports.wit");
wit_bindgen_guest_rust::export!("../../tests/runtime/unions/exports.wit");

use exports::*;

struct Exports;

impl exports::Exports for Exports {
    fn test_imports() {
        use imports::*;

        // All-Integers
        // Booleans
        assert!(matches!(
            add_one_integer(AllIntegers::Bool(false)),
            AllIntegers::Bool(true)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::Bool(true)),
            AllIntegers::Bool(false)
        ));
        // Unsigned integers
        assert!(matches!(
            add_one_integer(AllIntegers::U8(0)),
            AllIntegers::U8(1)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::U8(u8::MAX)),
            AllIntegers::U8(0)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::U16(0)),
            AllIntegers::U16(1)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::U16(u16::MAX)),
            AllIntegers::U16(0)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::U32(0)),
            AllIntegers::U32(1)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::U32(u32::MAX)),
            AllIntegers::U32(0)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::U64(0)),
            AllIntegers::U64(1)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::U64(u64::MAX)),
            AllIntegers::U64(0)
        ));
        // Signed integers
        assert!(matches!(
            add_one_integer(AllIntegers::I8(0)),
            AllIntegers::I8(1)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::I8(i8::MAX)),
            AllIntegers::I8(i8::MIN)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::I16(0)),
            AllIntegers::I16(1)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::I16(i16::MAX)),
            AllIntegers::I16(i16::MIN)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::I32(0)),
            AllIntegers::I32(1)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::I32(i32::MAX)),
            AllIntegers::I32(i32::MIN)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::I64(0)),
            AllIntegers::I64(1)
        ));
        assert!(matches!(
            add_one_integer(AllIntegers::I64(i64::MAX)),
            AllIntegers::I64(i64::MIN)
        ));

        // All-Floats
        assert!(
            matches!(add_one_float(AllFloats::F32(0.0)), AllFloats::F32(x) if x - 1.0 < f32::EPSILON)
        );
        assert!(
            matches!(add_one_float(AllFloats::F64(0.0)), AllFloats::F64(x) if x - 1.0 < f64::EPSILON)
        );

        // All-Text
        assert!(matches!(
            replace_first_char(AllTextParam::Char('a'), 'z'),
            AllTextResult::Char('z')
        ));
        assert!(
            matches!(replace_first_char(AllTextParam::String("abc"), 'z'), AllTextResult::String(r) if r == "zbc")
        );

        // All-Integers
        assert!(matches!(identify_integer(AllIntegers::Bool(true)), 0));
        assert!(matches!(identify_integer(AllIntegers::U8(0)), 1));
        assert!(matches!(identify_integer(AllIntegers::U16(0)), 2));
        assert!(matches!(identify_integer(AllIntegers::U32(0)), 3));
        assert!(matches!(identify_integer(AllIntegers::U64(0)), 4));
        assert!(matches!(identify_integer(AllIntegers::I8(0)), 5));
        assert!(matches!(identify_integer(AllIntegers::I16(0)), 6));
        assert!(matches!(identify_integer(AllIntegers::I32(0)), 7));
        assert!(matches!(identify_integer(AllIntegers::I64(0)), 8));

        // All-Floats
        assert!(matches!(identify_float(AllFloats::F32(0.0)), 0));
        assert!(matches!(identify_float(AllFloats::F64(0.0)), 1));

        // All-Text
        assert!(matches!(identify_text(AllTextParam::Char('a')), 0));
        assert!(matches!(identify_text(AllTextParam::String("abc")), 1));

        // Duplicated
        assert!(matches!(
            add_one_duplicated(DuplicatedS32::I320(0)),
            DuplicatedS32::I320(1)
        ));
        assert!(matches!(
            add_one_duplicated(DuplicatedS32::I321(1)),
            DuplicatedS32::I321(2)
        ));
        assert!(matches!(
            add_one_duplicated(DuplicatedS32::I322(2)),
            DuplicatedS32::I322(3)
        ));

        assert!(matches!(identify_duplicated(DuplicatedS32::I320(0)), 0));
        assert!(matches!(identify_duplicated(DuplicatedS32::I321(0)), 1));
        assert!(matches!(identify_duplicated(DuplicatedS32::I321(0)), 2));

        // Distinguishable
        assert!(
            matches!(add_one_distinguishable_num(DistinguishableNum::F64(0.0)), DistinguishableNum::F64(x) if x - 1.0 < f64::EPSILON)
        );
        assert!(matches!(
            add_one_distinguishable_num(DistinguishableNum::I64(0)),
            DistinguishableNum::I64(1)
        ));

        assert!(matches!(
            identify_distinguishable_num(DistinguishableNum::F64(0.0)),
            0
        ));
        assert!(matches!(
            identify_distinguishable_num(DistinguishableNum::I64(1)),
            1
        ));
    }

    fn add_one_integer(num: AllIntegers) -> AllIntegers {
        match num {
            // Boolean
            AllIntegers::Bool(b) => AllIntegers::Bool(!b),
            // Unsigned Integers
            AllIntegers::U8(n) => AllIntegers::U8(n.wrapping_add(1)),
            AllIntegers::U16(n) => AllIntegers::U16(n.wrapping_add(1)),
            AllIntegers::U32(n) => AllIntegers::U32(n.wrapping_add(1)),
            AllIntegers::U64(n) => AllIntegers::U64(n.wrapping_add(1)),
            // Signed Integers
            AllIntegers::I8(n) => AllIntegers::I8(n.wrapping_add(1)),
            AllIntegers::I16(n) => AllIntegers::I16(n.wrapping_add(1)),
            AllIntegers::I32(n) => AllIntegers::I32(n.wrapping_add(1)),
            AllIntegers::I64(n) => AllIntegers::I64(n.wrapping_add(1)),
        }
    }

    fn add_one_float(num: AllFloats) -> AllFloats {
        match num {
            AllFloats::F32(n) => AllFloats::F32(n + 1.0),
            AllFloats::F64(n) => AllFloats::F64(n + 1.0),
        }
    }

    fn replace_first_char(text: AllText, letter: char) -> AllText {
        match text {
            AllText::Char(_c) => AllText::Char(letter),
            AllText::String(s) => AllText::String(format!("{}{}", letter, &s[1..])),
        }
    }

    fn identify_integer(num: AllIntegers) -> u8 {
        match num {
            // Boolean
            AllIntegers::Bool(_b) => 0,
            // Unsigned Integers
            AllIntegers::U8(_n) => 1,
            AllIntegers::U16(_n) => 2,
            AllIntegers::U32(_n) => 3,
            AllIntegers::U64(_n) => 4,
            // Signed Integers
            AllIntegers::I8(_n) => 5,
            AllIntegers::I16(_n) => 6,
            AllIntegers::I32(_n) => 7,
            AllIntegers::I64(_n) => 8,
        }
    }

    fn identify_float(num: AllFloats) -> u8 {
        match num {
            AllFloats::F32(_n) => 0,
            AllFloats::F64(_n) => 1,
        }
    }

    fn identify_text(text: AllText) -> u8 {
        match text {
            AllText::Char(_c) => 0,
            AllText::String(_s) => 1,
        }
    }

    fn add_one_duplicated(num: DuplicatedS32) -> DuplicatedS32 {
        match num {
            DuplicatedS32::I320(n) => DuplicatedS32::I320(n.wrapping_add(1)),
            DuplicatedS32::I321(n) => DuplicatedS32::I321(n.wrapping_add(1)),
            DuplicatedS32::I322(n) => DuplicatedS32::I322(n.wrapping_add(1)),
        }
    }

    fn identify_duplicated(num: DuplicatedS32) -> u8 {
        match num {
            DuplicatedS32::I320(_n) => 0,
            DuplicatedS32::I321(_n) => 1,
            DuplicatedS32::I322(_n) => 2,
        }
    }

    fn add_one_distinguishable_num(num: DistinguishableNum) -> DistinguishableNum {
        match num {
            DistinguishableNum::F64(n) => DistinguishableNum::F64(n + 1.0),
            DistinguishableNum::I64(n) => DistinguishableNum::I64(n.wrapping_add(1)),
        }
    }

    fn identify_distinguishable_num(num: DistinguishableNum) -> u8 {
        match num {
            DistinguishableNum::F64(_n) => 0,
            DistinguishableNum::I64(_n) => 1,
        }
    }
}
