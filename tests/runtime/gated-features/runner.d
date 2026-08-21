//@ args = '--features y'

import wit.foo.bar.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    y();
    z();
}

alias Exports = wit.foo.bar.runner.Exports!(
    run
);
