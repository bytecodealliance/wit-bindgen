import wit.test.versions.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    import v1 = wit.test.dep_0_1_0.test.imports;

    assert(v1.x() == 1.0);
    assert(v1.y(1.0) == 2.0);

    import v2 = wit.test.dep_0_2_0.test.imports;
    assert(v2.x() == 2.0);
    assert(v2.z(1.0, 1.0) == 4.0);
}

alias Exports = wit.test.versions.runner.Exports!(
    run
);
