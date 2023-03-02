use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!(in "tests/runtime/wildcards");

#[derive(Default)]
struct Host;

impl imports::Host for Host {}

struct Match(u32);

impl imports::WildcardMatch<Host> for Match {
    fn call(&self, _host: &mut Host, _name: &str) -> Result<u32> {
        Ok(self.0)
    }
}

#[test]
fn run() -> Result<()> {
    eprintln!("yossa hello");
    crate::run_test(
        "wildcards",
        |linker| {
            eprintln!("yossa add to linker");
            Wildcards::add_to_linker(
                linker,
                WildcardMatches {
                    imports: vec![("a", Match(42)), ("b", Match(43)), ("c", Match(44))],
                },
                |x| &mut x.0,
            )
        },
        |store, component, linker| Wildcards::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(wildcards: Wildcards, store: &mut Store<crate::Wasi<Host>>) -> Result<()> {
    for (name, value) in [("x", 42), ("y", 43), ("z", 44)] {
        assert_eq!(
            value,
            wildcards
                .exports
                .get_wildcard_match(name)
                .unwrap()
                .call(&mut *store)?
        );
    }

    Ok(())
}
