import wit.a.b.runner;
import wit.common;

@witInterface("$root")
@witExport
void run() {
    x();
}

alias Exports = wit.a.b.runner.Exports!(
    run
);
