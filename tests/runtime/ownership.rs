use anyhow::Result;
use wasmtime::component::Resource;
use wasmtime::Store;

wasmtime::component::bindgen!({
    path: "tests/runtime/ownership",
    with: {
        "test:ownership/both-list-and-resource/the-resource": MyResource,
    },
});

#[derive(Default)]
pub struct MyImports {
    called_foo: bool,
    called_bar: bool,
    called_baz: bool,
    last_resource_list: Option<Vec<String>>,
}

pub struct MyResource;

impl lists::Host for MyImports {
    fn foo(&mut self, list: Vec<Vec<String>>) -> Vec<Vec<String>> {
        self.called_foo = true;
        list
    }
}

impl thing_in::Host for MyImports {
    fn bar(&mut self, _value: thing_in::Thing) {
        self.called_bar = true;
    }
}

impl thing_in_and_out::Host for MyImports {
    fn baz(&mut self, value: thing_in_and_out::Thing) -> thing_in_and_out::Thing {
        self.called_baz = true;
        value
    }
}

impl test::ownership::both_list_and_resource::Host for MyImports {
    fn list_and_resource(&mut self, value: test::ownership::both_list_and_resource::Thing) {
        assert_eq!(value.b.rep(), 100);
        assert!(value.b.owned());
        let expected = self.last_resource_list.as_ref().unwrap();
        assert_eq!(value.a, *expected);
    }
}

impl test::ownership::both_list_and_resource::HostTheResource for MyImports {
    fn new(&mut self, list: Vec<String>) -> Resource<MyResource> {
        assert!(self.last_resource_list.is_none());
        self.last_resource_list = Some(list);
        Resource::new_own(100)
    }

    fn drop(&mut self, _val: Resource<MyResource>) -> Result<()> {
        unreachable!()
    }
}

#[test]
fn run() -> Result<()> {
    for name in ["owning", "borrowing", "borrowing-duplicate-if-necessary"] {
        crate::run_test_from_dir(
            "ownership",
            name,
            |linker| Ownership::add_to_linker(linker, |x| &mut x.0),
            |store, component, linker| Ownership::instantiate(store, component, linker),
            run_test,
        )?;
    }

    Ok(())
}

fn run_test(exports: Ownership, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_foo(&mut *store)?;

    assert!(store.data().0.called_foo);
    assert!(store.data().0.called_bar);
    assert!(store.data().0.called_baz);
    assert!(store.data().0.last_resource_list.is_some());

    Ok(())
}
