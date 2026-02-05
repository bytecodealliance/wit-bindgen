//@ args = ['--merge-structurally-equal-types', '-dPartialEq', '--additional-derive-ignore=kind7', '--additional-derive-ignore=kind3', '--additional-derive-ignore=kind4', '--additional-derive-ignore=t-stream']

include!(env!("BINDINGS"));

use crate::test::equal_types::{blag, blah};

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        let kind1 = blag::Kind1::A;
        let res1 = blag::f(kind1);
        let kind6 = blah::Kind6::A;
        let res2 = blah::f(kind6);
        assert_eq!(res1, res2);
        let t2 = blag::T2 {
            l: blag::T3 { l: kind1.clone(), r: kind1.clone() },
            r: blah::T3 { l: kind1.clone(), r: kind1.clone() }
        };
        let t1 = blag::T1 {
            l: t2.clone(),
            r: t2.clone(),
        };
        let t = blag::Tree {
            l: t1.clone(),
            r: t1.clone(),
        };
        let t_stream = blag::TStream { tree: t.clone(), stream: None };
        let res1 = blag::h(&t_stream);
        let blah::CustomResult::Ok(res2) = blah::h(&t_stream) else { unreachable!() };
        assert_eq!(res1, res2);
    }
}
