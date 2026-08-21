import wit.foo.bar.test;

import wit.foo.bar.component.common : UnusedEnum, UnusedRecord, UnusedVariant;

import wit.common;

@witExport("foo:bar/component", "foo")
void foo() {}

alias Exports = wit.foo.bar.test.Exports!(
    foo
);
