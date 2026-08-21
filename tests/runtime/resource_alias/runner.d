import wit.test.resource_alias.runner;

import wit.test.resource_alias.e1.imports : a1 = a, Foo1 = Foo, X;
import wit.test.resource_alias.e2.imports : a2 = a, Foo2 = Foo;

import wit.common;

@witExport("$root", "run")
void run() {
    auto fooE1 = Foo1(
        x: X.makeNew(42)
    );

    // consumed later

    a1(fooE1);

    auto fooE2 = Foo2(
        x: X.makeNew(7)
    );
    // consumed later

    auto barE2 = Foo1(
        x: X.makeNew(8)
    );
    // consumed later

    auto y = X.makeNew(8);
    scope(exit) y.witDrop;

    a2(fooE2, barE2, y);
}

alias Exports = wit.test.resource_alias.runner.Exports!(
    run
);
