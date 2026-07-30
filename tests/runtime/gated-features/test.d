//@ args = '--features y'

import wit.foo.bar.test;
import wit.common;

@witExport("foo:bar/bindings@1.2.3", "y")
void y() {}

@witExport("foo:bar/bindings@1.2.3", "z")
void z() {}

alias Exports = wit.foo.bar.test.Exports!(
    y,
    z
);
