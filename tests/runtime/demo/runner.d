import wit.a.b.runner;
import wit.common;

@witInterface("$root")
@witExport("run")
void run() {
    x();
}

alias Exports = wit.a.b.runner.Exports!(
    run
);
