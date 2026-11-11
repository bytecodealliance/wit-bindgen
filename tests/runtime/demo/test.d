import wit.a.b.test;
import wit.common;

@witInterface("a:b/the-test")
@witExport
void x() {
}

alias Exports = wit.a.b.test.Exports!(
    x
);
