wit_bindgen_rust::import!("../../tests/runtime/unions/imports.wit");
wit_bindgen_rust::export!("../../tests/runtime/unions/exports.wit");

use exports::*;

struct Exports;

impl exports::Exports for Exports {
    fn test_imports() {
        use imports::*;

        // All-Integers
        // Booleans
        assert!(matches!(add_one_integer(AllIntegers::V0(false)), AllIntegers::V0(true)));
        assert!(matches!(add_one_integer(AllIntegers::V0(true)), AllIntegers::V0(false)));
        // Unsigned integers
        assert!(matches!(add_one_integer(AllIntegers::V1(0)), AllIntegers::V1(1)));
        assert!(matches!(add_one_integer(AllIntegers::V1(u8::MAX)), AllIntegers::V1(0)));
        assert!(matches!(add_one_integer(AllIntegers::V2(0)), AllIntegers::V2(1)));
        assert!(matches!(add_one_integer(AllIntegers::V2(u16::MAX)), AllIntegers::V2(0)));
        assert!(matches!(add_one_integer(AllIntegers::V3(0)), AllIntegers::V3(1)));
        assert!(matches!(add_one_integer(AllIntegers::V3(u32::MAX)), AllIntegers::V3(0)));
        assert!(matches!(add_one_integer(AllIntegers::V4(0)), AllIntegers::V4(1)));
        assert!(matches!(add_one_integer(AllIntegers::V4(u64::MAX)), AllIntegers::V4(0)));
        // Signed integers
        assert!(matches!(add_one_integer(AllIntegers::V5(0)), AllIntegers::V5(1)));
        assert!(matches!(add_one_integer(AllIntegers::V5(i8::MAX)), AllIntegers::V5(i8::MIN)));
        assert!(matches!(add_one_integer(AllIntegers::V6(0)), AllIntegers::V6(1)));
        assert!(matches!(add_one_integer(AllIntegers::V6(i16::MAX)), AllIntegers::V6(i16::MIN)));
        assert!(matches!(add_one_integer(AllIntegers::V7(0)), AllIntegers::V7(1)));
        assert!(matches!(add_one_integer(AllIntegers::V7(i32::MAX)), AllIntegers::V7(i32::MIN)));
        assert!(matches!(add_one_integer(AllIntegers::V8(0)), AllIntegers::V8(1)));
        assert!(matches!(add_one_integer(AllIntegers::V8(i64::MAX)), AllIntegers::V8(i64::MIN)));

        // All-Floats
        assert!(matches!(add_one_float(AllFloats::V0(0.0)), AllFloats::V0(1.0)));
        assert!(matches!(add_one_float(AllFloats::V1(0.0)), AllFloats::V1(1.0)));

        // All-Text
        assert!(matches!(replace_first_char(AllTextParam::V0('a'), 'z'), AllTextResult::V0('z')));
        let rhs = "zbc".to_string();
        assert!(matches!(replace_first_char(AllTextParam::V1("abc"), 'z'), AllTextResult::V1(rhs)));

        // All-Integers
        assert!(matches!(identify_integer(AllIntegers::V0(true)), 0));
        assert!(matches!(identify_integer(AllIntegers::V1(0)), 1));
        assert!(matches!(identify_integer(AllIntegers::V2(0)), 2));
        assert!(matches!(identify_integer(AllIntegers::V3(0)), 3));
        assert!(matches!(identify_integer(AllIntegers::V4(0)), 4));
        assert!(matches!(identify_integer(AllIntegers::V5(0)), 5));
        assert!(matches!(identify_integer(AllIntegers::V6(0)), 6));
        assert!(matches!(identify_integer(AllIntegers::V7(0)), 7));
        assert!(matches!(identify_integer(AllIntegers::V8(0)), 8));

        // All-Floats
        assert!(matches!(identify_float(AllFloats::V0(0.0)), 0));
        assert!(matches!(identify_float(AllFloats::V1(0.0)), 1));

        // All-Text
        assert!(matches!(identify_text(AllTextParam::V0('a')), 0));
        assert!(matches!(identify_text(AllTextParam::V1("abc")), 1));

        // Duplicated
        assert!(matches!(add_one_duplicated(DuplicatedS32::V0(0)), DuplicatedS32::V0(1)));
        assert!(matches!(add_one_duplicated(DuplicatedS32::V1(1)), DuplicatedS32::V1(2)));
        assert!(matches!(add_one_duplicated(DuplicatedS32::V2(2)), DuplicatedS32::V2(3)));

        assert!(matches!(identify_duplicated(DuplicatedS32::V0(0)), 0));
        assert!(matches!(identify_duplicated(DuplicatedS32::V1(0)), 1));
        assert!(matches!(identify_duplicated(DuplicatedS32::V2(0)), 2));

        // Distinguishable
        assert!(matches!(add_one_distinguishable_num(DistinguishableNum::V0(0.0)), DistinguishableNum::V0(1.0)));
        assert!(matches!(add_one_distinguishable_num(DistinguishableNum::V1(0)), DistinguishableNum::V1(1)));

        assert!(matches!(identify_distinguishable_num(DistinguishableNum::V0(0.0)), 0));
        assert!(matches!(identify_distinguishable_num(DistinguishableNum::V1(1)), 1));
    }

    fn add_one_integer(num: AllIntegers) -> AllIntegers {
        match num {
            // Boolean
            AllIntegers::V0(b) => AllIntegers::V0(!b),
            // Unsigneed Integers
            AllIntegers::V1(n) => AllIntegers::V1(n.wrapping_add(1)),
            AllIntegers::V2(n) => AllIntegers::V2(n.wrapping_add(1)),
            AllIntegers::V3(n) => AllIntegers::V3(n.wrapping_add(1)),
            AllIntegers::V4(n) => AllIntegers::V4(n.wrapping_add(1)),
            // Signed Integers
            AllIntegers::V5(n) => AllIntegers::V5(n.wrapping_add(1)),
            AllIntegers::V6(n) => AllIntegers::V6(n.wrapping_add(1)),
            AllIntegers::V7(n) => AllIntegers::V7(n.wrapping_add(1)),
            AllIntegers::V8(n) => AllIntegers::V8(n.wrapping_add(1)),
        }
    }

    fn add_one_float(num: AllFloats) -> AllFloats {
        match num {
            AllFloats::V0(n) => AllFloats::V0(n + 1.0),
            AllFloats::V1(n) => AllFloats::V1(n + 1.0),
        }
    }

    fn replace_first_char(text: AllText, letter: char) -> AllText {
        match text {
            AllText::V0(c) => AllText::V0(letter),
            AllText::V1(s) => AllText::V1(format!("{}{}", letter, &s[1..]))
        }
    }

    fn identify_integer(num: AllIntegers) -> u8 {
        match num {
            // Boolean
            AllIntegers::V0(_b) => 0,
            // Unsigneed Integers
            AllIntegers::V1(_n) => 1,
            AllIntegers::V2(_n) => 2,
            AllIntegers::V3(_n) => 3,
            AllIntegers::V4(_n) => 4,
            // Signed Integers
            AllIntegers::V5(_n) => 5,
            AllIntegers::V6(_n) => 6,
            AllIntegers::V7(_n) => 7,
            AllIntegers::V8(_n) => 8,
        }
    }

    fn identify_float(num: AllFloats) -> u8 {
        match num {
            AllFloats::V0(_n) => 0,
            AllFloats::V1(_n) => 1,
        }
    }

    fn identify_text(text: AllText) -> u8 {
        match text {
            AllText::V0(_c) => 0,
            AllText::V1(_s) => 1
        }
    }

    fn add_one_duplicated(num: DuplicatedS32) -> DuplicatedS32 {
        match num {
            DuplicatedS32::V0(n) => DuplicatedS32::V0(n.wrapping_add(1)),
            DuplicatedS32::V1(n) => DuplicatedS32::V1(n.wrapping_add(1)),
            DuplicatedS32::V2(n) => DuplicatedS32::V2(n.wrapping_add(1)),
        }
    }

    fn identify_duplicated(num: DuplicatedS32) -> u8 {
        match num {
            DuplicatedS32::V0(_n) => 0,
            DuplicatedS32::V1(_n) => 1,
            DuplicatedS32::V2(_n) => 2,
        }
    }

    fn add_one_distinguishable_num(num: DistinguishableNum) -> DistinguishableNum {
        match num {
            DistinguishableNum::V0(n) => DistinguishableNum::V0(n + 1.0),
            DistinguishableNum::V1(n) => DistinguishableNum::V1(n.wrapping_add(1)),
        }
    }

    fn identify_distinguishable_num(num: DistinguishableNum) -> u8 {
        match num {
            DistinguishableNum::V0(_n) => 0,
            DistinguishableNum::V1(_n) => 1,
        }
    }
}
