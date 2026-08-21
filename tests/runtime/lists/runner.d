import wit.test.lists.runner;
import wit.common;

extern extern(C) size_t walloc_allocated_bytes;

extern(C) void* malloc(size_t size);
extern(C) void free(void* ptr);

@witExport("$root", "run")
void run() {
    auto allocedAtFuncStart = walloc_allocated_bytes;
    auto allocedAtFuncStart2 = allocatedBytes;
    scope(exit) assert(
        walloc_allocated_bytes == allocedAtFuncStart
        && allocatedBytes == allocedAtFuncStart2
    );

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        emptyListParam(WitList!ubyte());
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        emptyStringParam("".witList);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        assert(!emptyListResult().length);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        assert(!emptyStringResult().length);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable ubyte[4] inputs = [1, 2, 3, 4];
        listParam(inputs.witList);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        listParam2("foo".witList);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        immutable WitString[3] inputs = ["foo".witList, "bar".witList, "baz".witList];
        listParam3(inputs.witList);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        immutable WitString[2] inputs = ["foo".witList, "bar".witList];
        immutable WitString[1] inputs2 = ["baz".witList];

        immutable WitList!WitString[2] inputs3 = [inputs.witList, inputs2.witList];
        listParam4(inputs3.witList);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        immutable Tuple!(ubyte, uint, ubyte)[2] inputs = [
            tuple(ubyte(1), uint(2), ubyte(3)),
            tuple(ubyte(4), uint(5), ubyte(6))
        ];

        listParam5(inputs.witList);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        enum len = 1000;
        auto ptr = cast(WitString*)malloc(WitString.sizeof*len);
        assert(ptr);
        scope(exit) free(ptr);

        foreach (ref str; ptr[0..len]) {
            str = cast()"string".witList;
        }

        listParamLarge(ptr[0..len].witList);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        auto result = listResult();
        scope(exit) result.witFree;

        static immutable ubyte[5] outputs = [1, 2, 3, 4, 5];
        assert(result == outputs);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        auto result = listResult2();
        scope(exit) result.witFree;

        assert(result == "hello!");
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        auto result = listResult3();
        scope(exit) result.witFree;

        immutable WitString[2] outputs = ["hello,".witList, "world!".witList];
        assert(result == outputs);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable ubyte[0] inputs = [];

        auto result = listRoundtrip(inputs.witList);
        scope(exit) result.witFree;

        assert(result == inputs);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable ubyte[1] inputs = ['x'];

        auto result = listRoundtrip(inputs.witList);
        scope(exit) result.witFree;

        assert(result == inputs);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable ubyte[5] inputs = ['h', 'e', 'l', 'l', 'o'];

        auto result = listRoundtrip(inputs.witList);
        scope(exit) result.witFree;

        assert(result == inputs);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable string input = "x";

        auto result = stringRoundtrip(input.witList);
        scope(exit) result.witFree;

        assert(result == input);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable string input = "";

        auto result = stringRoundtrip(input.witList);
        scope(exit) result.witFree;

        assert(result == input);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable string input = "hello";

        auto result = stringRoundtrip(input.witList);
        scope(exit) result.witFree;

        assert(result == input);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable string input = "hello ⚑ world";

        auto result = stringRoundtrip(input.witList);
        scope(exit) result.witFree;

        assert(result == input);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable ubyte[2] inputs1 = [ubyte.min, ubyte.max];
        static immutable byte[2] inputs2 = [byte.min, byte.max];

        auto result = listMinmax8(inputs1.witList, inputs2.witList);
        scope(exit) result.witFree;

        assert(result[0] == inputs1);
        assert(result[1] == inputs2);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable ushort[2] inputs1 = [ushort.min, ushort.max];
        static immutable short[2] inputs2 = [short.min, short.max];

        auto result = listMinmax16(inputs1.witList, inputs2.witList);
        scope(exit) result.witFree;

        assert(result[0] == inputs1);
        assert(result[1] == inputs2);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable uint[2] inputs1 = [uint.min, uint.max];
        static immutable int[2] inputs2 = [int.min, int.max];

        auto result = listMinmax32(inputs1.witList, inputs2.witList);
        scope(exit) result.witFree;

        assert(result[0] == inputs1);
        assert(result[1] == inputs2);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable ulong[2] inputs1 = [ulong.min, ulong.max];
        static immutable long[2] inputs2 = [long.min, long.max];

        auto result = listMinmax64(inputs1.witList, inputs2.witList);
        scope(exit) result.witFree;

        assert(result[0] == inputs1);
        assert(result[1] == inputs2);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable float[2] inputs1 = [-float.infinity, float.infinity];
        static immutable double[2] inputs2 = [-double.infinity, double.infinity];

        auto result = listMinmaxFloat(inputs1.witList, inputs2.witList);
        scope(exit) result.witFree;

        assert(result[0] == inputs1);
        assert(result[1] == inputs2);
    }

    {
        auto allocedAtScopeStart = walloc_allocated_bytes;
        auto allocedAtScopeStart2 = allocatedBytes;
        scope(exit) assert(
            walloc_allocated_bytes == allocedAtScopeStart
            && allocatedBytes == allocedAtScopeStart2
        );

        static immutable ubyte[10] textPlain = ['t', 'e', 'x', 't', '/', 'p', 'l', 'a', 'i', 'n'];
        static immutable ubyte[9] notFound = ['N', 'o', 't', ' ', 'f', 'o', 'u', 'n', 'd'];

        immutable Tuple!(WitString, WitList!ubyte)[2] headers = [
            tuple("Content-Type".witList, textPlain.witList),
            tuple("Content-Length".witList, notFound.witList)
        ];

        auto result = wasiHttpHeadersRoundtrip(headers.witList);
        scope(exit) result.witFree;

        assert(result[0][0] == "Content-Type");
        assert(result[0][1] == textPlain);
        assert(result[1][0] == "Content-Length");
        assert(result[1][1] == notFound);
    }
}

alias Exports = wit.test.lists.runner.Exports!(
    run
);
