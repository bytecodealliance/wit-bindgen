import wit.my.inline.runner;

import wit.common;

@witExport("$root", "run")
void run() {
    wit.my.inline.foo1.imports.foo();
    wit.my.inline.foo2.imports.foo();
    wit.my.inline.bar1.imports.bar();
    wit.my.inline.bar2.imports.bar();
}

alias Exports = wit.my.inline.runner.Exports!(
    run
);
