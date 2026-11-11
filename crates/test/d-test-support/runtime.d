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

void _d_array_slice_copy(void* dst, size_t dstlen, void* src, size_t srclen, size_t elemsz)
{
    import ldc.intrinsics : llvm_memcpy;

    //enforceRawArraysConformable("copy", elemsz, src[0..srclen], dst[0..dstlen]);
    assert(srclen == dstlen);

    llvm_memcpy!size_t(dst, src, dstlen * elemsz, 0);
}
