import wit.test.common.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    R1 res = wrap(F1.a);
    assert(res.b == F1.a);
    assert(res.a == 1);

    R1 res2 = wrap(F1.b);
    assert(res2.b == F1.b);
    assert(res2.a == 2);

    V1 res3 = varF();
    assert(res3.isB);
    assert(res3.getB == 42);
}

alias Exports = wit.test.common.runner.Exports!(
    run
);
