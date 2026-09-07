//@ args = '--features y'

import wit.foo.bar.runner;
import wit.common;

@witInterface("$root")
@witExport
void run() {
    y();
    z();
}

alias Exports = wit.foo.bar.runner.Exports!(
    run
);
