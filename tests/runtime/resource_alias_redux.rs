use std::collections::HashMap;
use std::vec;
use wasmtime::{component::Resource, Store};

use exports::test::resource_alias_redux::resource_alias1::Foo as ExportFoo1;
use exports::test::resource_alias_redux::resource_alias2::Foo as ExportFoo2;

use self::test::resource_alias_redux::resource_alias1::{Foo as Foo1, Host as Host1, HostThing};
use self::test::resource_alias_redux::resource_alias2::{Bar, Foo as Foo2, Host as Host2};

wasmtime::component::bindgen!(in "tests/runtime/resource_alias_redux");

#[derive(Default)]
pub struct MyHost {
    map_l: HashMap<u32, String>,
    next_id: u32,
}

impl MyHost {
    pub fn get_value(&self, id: u32) -> String {
        self.map_l[&id].clone()
    }
}

impl HostThing for MyHost {
    fn new(&mut self, s: String) -> wasmtime::component::Resource<Thing> {
        let id = self.next_id;
        self.next_id += 1;
        self.map_l.insert(id, s + " HostThing");
        Resource::new_own(id)
    }

    fn get(&mut self, self_: wasmtime::component::Resource<Thing>) -> String {
        let id = self_.rep();
        self.map_l[&id].clone() + " HostThing.get"
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Thing>) -> wasmtime::Result<()> {
        let id = rep.rep();
        self.map_l.remove(&id);
        Ok(())
    }
}

impl Host1 for MyHost {
    fn a(&mut self, f: Foo1) -> Vec<wasmtime::component::Resource<Thing>> {
        vec![f.thing]
    }
}

impl Host2 for MyHost {
    fn b(&mut self, f: Foo2, g: Bar) -> Vec<wasmtime::component::Resource<Thing>> {
        vec![f.thing, g.thing]
    }
}

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_alias_redux",
        |linker| ResourceAliasRedux::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| {
            let (u, e) = ResourceAliasRedux::instantiate(store, component, linker)?;
            Ok((u, e))
        },
        run_test,
    )
}

fn run_test(
    instance: ResourceAliasRedux,
    store: &mut Store<crate::Wasi<MyHost>>,
) -> anyhow::Result<()> {
    let mut thing = MyHost::default();
    let thing1 = HostThing::new(&mut thing, "Ni Hao".to_string());
    let res: Vec<String> = instance
        .call_test(&mut *store, &[thing1])?
        .into_iter()
        .map(|x| {
            let val = thing.get_value(x.rep());
            let _ = thing.drop(x);
            val
        })
        .collect();
    assert_eq!(res, vec!["Ni Hao HostThing"]);

    let thing2 = instance
        .test_resource_alias_redux_resource_alias1()
        .thing()
        .call_constructor(&mut *store, "Ciao")?;
    let res: Vec<_> = instance
        .test_resource_alias_redux_resource_alias1()
        .call_a(&mut *store, ExportFoo1 { thing: thing2 })?
        .into_iter()
        .filter_map(|x| {
            instance
                .test_resource_alias_redux_resource_alias1()
                .thing()
                .call_get(&mut *store, x)
                .ok()
        })
        .collect();
    assert_eq!(res, vec!["Ciao Thing HostThing HostThing.get Thing.get"]);

    let thing3 = instance
        .test_resource_alias_redux_resource_alias1()
        .thing()
        .call_constructor(&mut *store, "Ciao")?;
    let thing4 = instance
        .test_resource_alias_redux_resource_alias1()
        .thing()
        .call_constructor(&mut *store, "Aloha")?;
    let res: Vec<_> = instance
        .test_resource_alias_redux_resource_alias2()
        .call_b(
            &mut *store,
            ExportFoo2 { thing: thing3 },
            ExportFoo1 { thing: thing4 },
        )?
        .into_iter()
        .filter_map(|x| {
            instance
                .test_resource_alias_redux_resource_alias1()
                .thing()
                .call_get(&mut *store, x)
                .ok()
        })
        .collect();
    assert_eq!(
        res,
        vec![
            "Ciao Thing HostThing HostThing.get Thing.get",
            "Aloha Thing HostThing HostThing.get Thing.get"
        ]
    );
    Ok(())
}
