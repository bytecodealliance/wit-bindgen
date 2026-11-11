import wit.test.versions.test;
import wit.common;

@witInterface("test:dep/test@0.1.0") {
    @witExport("x")
    float x_v1() => 1.0;
    
    @witExport
    float y(float a) => 1.0 + a;
}

@witInterface("test:dep/test@0.2.0") {
    @witExport("x")
    float x_v2() => 2.0;
    
    @witExport
    float z(float a, float b) => 2.0 + a + b;
}

alias Exports = wit.test.versions.test.Exports!(
    x_v1,
    y,
    x_v2,
    z
);
