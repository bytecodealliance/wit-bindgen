//@ args = '--skip foo --std-feature'

#![no_std]

include!(env!("BINDINGS"));

struct Test;

export!(Test);

impl exports::exports::Guest for Test {
    fn bar() {}
}

#[unsafe(export_name = "exports#foo")]
pub extern "C" fn foo() {}
