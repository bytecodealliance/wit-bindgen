import wit.test.list_in_variant.test;
import wit.common;

// Allocates directly with `malloc`, so no witClone needed.
extern(C) void* malloc(size_t size);

char[] commaJoin(in WitString[] strs) {
    if (strs.length == 0) return null;

    size_t total = 0;
    foreach (i, str; strs) {
        total += str.length;

        if (i+1 != strs.length) {
            total += 1; // comma
        }
    }

    void* ptr = malloc(total);
    assert(ptr);
    char[] chars = cast(char[])ptr[0..total];

    size_t cursor = 0;
    foreach (i, str; strs) {
        foreach (chr; str) {
            chars[cursor++] = chr;
        }

        if (i+1 != strs.length) {
            chars[cursor++] = ',';
        }
    }

    return chars;
}

@witExport("test:list-in-variant/to-test", "list-in-option")
WitString listInOption(in Option!(WitList!WitString) data) {
    if (data.isSome) {
        return data.unwrap.commaJoin.witList; // no clone
    }
    return "none".witList.witClone;
}

@witExport("test:list-in-variant/to-test", "list-in-variant")
WitString listInVariant(in PayloadOrEmpty data) {
    if (data.isWithData) {
        return data.getWithData.commaJoin.witList; // no clone
    }
    return "empty".witList.witClone;
}

@witExport("test:list-in-variant/to-test", "list-in-result")
WitString listInResult(in Result!(WitList!WitString, WitString) data) {
    if (data.isOk) {
        return data.unwrap.commaJoin.witList;
    }


    auto errStr = data.unwrapErr;
    void* ptr = malloc(errStr.length+4);
    assert(ptr);
    char[] chars = cast(char[])ptr[0..errStr.length+4];

    chars[0..4] = "err:";
    foreach (i, ref chr; chars[4..$]) {
        chr = errStr[i];
    }

    return chars.witList; // no clone
}

@witExport("test:list-in-variant/to-test", "list-in-option-with-return")
Summary listInOptionWithReturn(in Option!(WitList!WitString) data) {
    if (data.isSome) {
        auto items = data.unwrap();
        return Summary(items.length, items.commaJoin.witList); // no clone
    }

    return Summary(0, "none".witList.witClone);
}

@witExport("test:list-in-variant/to-test", "top-level-list")
WitString topLevelList(in WitList!WitString data) {
    return data.commaJoin.witList; // no clone
}

alias Exports = wit.test.list_in_variant.test.Exports!(
    listInOption,
    listInVariant,
    listInResult,
    listInOptionWithReturn,
    topLevelList
);
