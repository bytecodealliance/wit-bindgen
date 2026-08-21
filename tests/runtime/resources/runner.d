import wit.test.resources.runner;

import wit.common;

@witExport("$root", "run")
void run() {
    {
        auto result = testImports();
        scope(exit) result.witFree;

        assert(result.isOk);
    }

    auto x = X.makeNew(5);
    scope(exit) x.witDrop;
    assert(x.getA == 5);
    x.setA(10);
    assert(x.getA == 10);

    auto z1 = Z.makeNew(10);
    scope(exit) z1.witDrop;
    assert(z1.getA() == 10);

    auto z2 = Z.makeNew(20);
    scope(exit) z2.witDrop;
    assert(z2.getA() == 20);

    auto xadd = X.add(x, 5);
    scope(exit) xadd.witDrop;
    x = X.init; // consumed
    assert(xadd.getA() == 15);

    auto zadd = add(z1, z2);
    scope(exit) zadd.witDrop;
    assert(zadd.getA() == 30);

    auto droppedZsStart = Z.numDropped;
    z1.witDrop;
    z2.witDrop;

    consume(xadd);
    xadd = X.init; // consumed

    auto droppedZsEnd = Z.numDropped;
    if (droppedZsStart != 0) {
        assert(droppedZsEnd == droppedZsStart + 2);
    }
}

alias Exports = wit.test.resources.runner.Exports!(
    run
);
