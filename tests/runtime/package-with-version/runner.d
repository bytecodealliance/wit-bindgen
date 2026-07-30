import wit.my.inline.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    Bar.makeNew().drop;
}

alias Exports = wit.my.inline.runner.Exports!(
    run
);
