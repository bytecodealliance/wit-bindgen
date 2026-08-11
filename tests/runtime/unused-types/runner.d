import wit.foo.bar.runner;

import wit.foo.bar.component.common : UnusedEnum, UnusedRecord, UnusedVariant;

import wit.common;

@witExport("$root", "run")
void run() {
    foo();
}

alias Exports = wit.foo.bar.runner.Exports!(
    run
);
