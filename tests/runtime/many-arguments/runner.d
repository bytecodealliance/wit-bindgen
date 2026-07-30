import wit.test.many_arguments.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    manyArguments(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
}

alias Exports = wit.test.many_arguments.runner.Exports!(
    run
);
