import wit.a.b.test;
import wit.common;

@witExport("a:b/the-test", "x")
void x() {
}

alias Exports = wit.a.b.test.Exports!(
    x
);
