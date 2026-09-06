import wit.my.inline.runner;
import wit.common;

@witInterface("$root")
@witExport("run")
void run() {
    Bar.makeNew().witDrop;
}

alias Exports = wit.my.inline.runner.Exports!(
    run
);
