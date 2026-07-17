extern(C) @nogc nothrow:

noreturn abort() {
    import ldc.intrinsics : llvm_trap;
    llvm_trap();
    while(true) {}
}

private int memcmp(const void* ptr1, const void* ptr2, size_t size)
{
    auto data1 = cast(const(ubyte)*)ptr1;
    auto data2 = cast(const(ubyte)*)ptr2;

    foreach (i; 0..size) {
        auto b1 = data1[i];
        auto b2 = data2[i];
        if (b1 != b2) return b1-b2;
    }

    return 0;
}
