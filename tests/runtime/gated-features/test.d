//@ args = '--features y'

import wit.foo.bar.test;
import wit.common;


@witInterface("foo:bar/bindings@1.2.3") {
    @witExport
    void y() {}
    
    @witExport
    void z() {}
}

alias Exports = wit.foo.bar.test.Exports!(
    y,
    z
);
