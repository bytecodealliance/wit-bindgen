import wit.a.b.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    x();
}

alias Exports = wit.a.b.runner.Exports!(
    run
);
