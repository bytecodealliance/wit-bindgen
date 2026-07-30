import wit.test.numbers.runner;
import wit.common;

void doAsserts(alias func)() {
    static if(is(typeof(func) P == function)) {
        alias T = P[0];
        static if (is(T == dchar)) {
            enum T a = 'a';
            enum T b = ' ';
            enum T c = '🚩';
        } else static if (__traits(isFloating, T)) {
            enum T a = 1.0;
            enum T b = -T.infinity;
            enum T c = T.infinity;
        } else {
            enum T a = 1;
            enum T b = T.min;
            enum T c = T.max;
        }
    }

    assert(func(a) == a);
    assert(func(b) == b);
    assert(func(c) == c);
}

@witExport("$root", "run")
void run() {
    doAsserts!roundtripU8;
    doAsserts!roundtripS8;
    doAsserts!roundtripU16;
    doAsserts!roundtripS16;
    doAsserts!roundtripU32;
    doAsserts!roundtripS32;
    doAsserts!roundtripU64;
    doAsserts!roundtripS64;
    doAsserts!roundtripF32;
    doAsserts!roundtripF64;
    doAsserts!roundtripChar;

    setScalar(2);
    assert(getScalar() == 2);

    setScalar(4);
    assert(getScalar() == 4);
}

alias Exports = wit.test.numbers.runner.Exports!(
    run
);
