//@ args = [
//@     '--with=my:inline/foo/a=crate::my_types::MyA',
//@     '--with=my:inline/foo/b=crate::my_types::MyB',
//@     '--with=my:inline/foo/c=crate::my_types::MyC',
//@     '--with=d=crate::my_types::MyD',
//@     '--with=my:inline/bar/e=crate::my_types::MyE',
//@     '--with=my:inline/foo/f=generate',
//@ ]

include!(env!("BINDINGS"));

mod my_types {
    #[derive(Debug, Clone, Copy)]
    pub struct MyA {
        pub inner: f64,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct MyB;

    impl MyB {
        pub fn take_handle(&self) -> u32 {
            0
        }

        pub fn from_handle(_handle: u32) -> Self {
            Self
        }
    }

    pub enum MyC {
        A(MyA),
        B(MyB),
    }

    pub struct MyD {
        pub inner: u32,
    }

    pub struct MyE {
        pub inner: u32,
    }
}

fn main() {
    let a = my_types::MyA { inner: 0.0 };
    let _ = my::inline::foo::func1(a);

    // can't actually succeed at runtime as this is faking a resource, so check
    // that it compiles but dynamically skip it.
    if false {
        let b = my_types::MyB;
        let _ = my::inline::foo::func2(b);
    }

    let c = my_types::MyC::A(a);
    let _ = i::func7(c);

    let a_list = vec![a, a];
    let _ = my::inline::foo::func3(&a_list);

    let _ = my::inline::foo::func4(Some(a));

    let _ = my::inline::foo::func5();

    let d = my_types::MyD { inner: 0 };
    let _ = i::func8(d);
}
