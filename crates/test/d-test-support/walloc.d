// From https://github.com/Inochi2D/numem/blob/main/modules/hookset-wasm/source/walloc.d
// Modified to include double-free detection

/**
    A small malloc implementation for use in WebAssembly targets

    Copyright (c) 2023-2025, Kitsunebi Games
    Copyright (c) 2023-2025, Inochi2D Project
    Copyright (c) 2020, Igalia, S.L.

    Distributed under an MIT-style License.
    (See accompanying LICENSE file or copy at
    https://github.com/wingo/walloc/blob/master/LICENSE.md)
*/

module walloc;
import ldc.intrinsics :
    llvm_wasm_memory_grow,
    llvm_wasm_memory_size,
    llvm_memmove;

extern(C) @nogc nothrow:

/// MODIFIED FOR wit-bindgen TESTS
enum MAX_ALLOCATIONS = 2048;
extern(D) void*[MAX_ALLOCATIONS] activePointers;
extern(D) size_t[MAX_ALLOCATIONS] activeAllocSizes;

// extern(C) to make it "public" for the `lists` test
extern(C) size_t walloc_allocated_bytes = 0;
/// END

void* malloc(size_t size) @nogc nothrow @system {
    if (size == 0)
        return null;

    size_t granules = size_to_granules(size);
    chunk_kind kind = granules_to_chunk_kind(granules);

    /// MODIFIED FOR wit-bindgen TESTS
    auto result = (kind == chunk_kind.LARGE_OBJECT) ? allocate_large(size) : allocate_small(kind);
    assert(result !is null);
    foreach (i, ref ptr; activePointers) {
        if (ptr !is null) continue;
        ptr = result;
        activeAllocSizes[i] = size;
        walloc_allocated_bytes += size;
        return result;
    }
    assert(0);
    /// END
}

export
void free(void *ptr) @nogc nothrow @system {
    /// MODIFIED FOR wit-bindgen TESTS
    assert(ptr !is null);

    bool found = false;
    foreach (i, ref existingPtr; activePointers) {
        if (ptr !is existingPtr) continue;
        existingPtr = null;
        walloc_allocated_bytes -= activeAllocSizes[i];
        found = true;
        break;
    }
    assert(found);
    /// END

    _page_t* page = get_page(ptr);
    size_t chunk = get_chunk_index(ptr);
    ubyte kind = page.header.chunk_kinds[chunk];
    if (kind == chunk_kind.LARGE_OBJECT) {
        _large_object_t* obj = get_large_object(ptr);
        obj.next = large_objects;
        large_objects = obj;
        allocate_chunk(page, chunk, chunk_kind.FREE_LARGE_OBJECT);
        pending_large_object_compact = 1;
    } else {
        size_t granules = kind;
        _freelist_t** loc = get_small_object_freelist(cast(chunk_kind)granules);
        _freelist_t* obj = cast(_freelist_t*)ptr;
        obj.next = *loc;
        *loc = obj;
    }
}

export
void* realloc(void* ptr, size_t newSize) @nogc nothrow @system {
    if (!ptr)
        return malloc(newSize);

    size_t oldSize = get_alloc_size(ptr);
    if (newSize <= oldSize)
        return ptr;

    // Size is bigger, realloc just to be sure.
    void* n_mem = malloc(newSize);
    llvm_memmove(n_mem, ptr, oldSize, true);
    free(ptr);
    return n_mem;
}

private:

size_t get_alloc_size(void* ptr) {
    _page_t* page = get_page(ptr);
    size_t chunk = get_chunk_index(ptr);
    chunk_kind kind = cast(chunk_kind)page.header.chunk_kinds[chunk];

    if (kind == chunk_kind.LARGE_OBJECT) {
        _large_object_t* obj = get_large_object(ptr);
        return obj.size;
    }

    if (kind < chunk_kind.SMALL_OBJECT_CHUNK_KINDS) {
        ptrdiff_t granules = chunk_kind_to_granules(kind);
        return granules * GRANULE_SIZE;
    }

    return 0;
}

extern __gshared void* __heap_base;
__gshared size_t walloc_heap_size;
__gshared _freelist_t*[chunk_kind.SMALL_OBJECT_CHUNK_KINDS] small_object_freelists;
__gshared _large_object_t* large_objects;


pragma(inline, true)
size_t _max(size_t a, size_t b) { return a < b ? b : a; }

pragma(inline, true)
size_t _alignv(size_t val, size_t alignment) { return (val + alignment - 1) & ~(alignment - 1); }

pragma(inline, true)
extern(D)
void __assert_aligned(T, Y)(T x, Y y) {
    assert(cast(size_t)x == _alignv(cast(size_t)x, cast(size_t)y));
}

enum size_t CHUNK_SIZE = 256;
enum size_t CHUNK_SIZE_LOG_2 = 8;
enum size_t CHUNK_MASK = (CHUNK_SIZE - 1);
enum size_t PAGE_SIZE = 65536;
enum size_t PAGE_SIZE_LOG_2 = 16;
enum size_t PAGE_MASK = (PAGE_SIZE - 1);
enum size_t CHUNKS_PER_PAGE = 256;
enum size_t GRANULE_SIZE = 8;
enum size_t GRANULE_SIZE_LOG_2 = 3;
enum size_t LARGE_OBJECT_THRESHOLD = 256;
enum size_t LARGE_OBJECT_GRANULE_THRESHOLD = 32;
enum size_t FIRST_ALLOCATABLE_CHUNK = 1;
enum size_t PAGE_HEADER_SIZE = _page_header_t.sizeof;
enum size_t LARGE_OBJECT_HEADER_SIZE = _large_object_t.sizeof;

static assert(PAGE_SIZE == CHUNK_SIZE * CHUNKS_PER_PAGE);
static assert(CHUNK_SIZE == 1 << CHUNK_SIZE_LOG_2);
static assert(PAGE_SIZE == 1 << PAGE_SIZE_LOG_2);
static assert(GRANULE_SIZE == 1 << GRANULE_SIZE_LOG_2);
static assert(LARGE_OBJECT_THRESHOLD ==
    LARGE_OBJECT_GRANULE_THRESHOLD * GRANULE_SIZE);

struct _chunk_t {
    void[CHUNK_SIZE] data;
}

enum chunk_kind : ubyte {
    GRANULES_1,
    GRANULES_2,
    GRANULES_3,
    GRANULES_4,
    GRANULES_5,
    GRANULES_6,
    GRANULES_8,
    GRANULES_10,
    GRANULES_16,
    GRANULES_32,

    SMALL_OBJECT_CHUNK_KINDS,
    FREE_LARGE_OBJECT = 254,
    LARGE_OBJECT = 255
}

__gshared const ubyte[] small_object_granule_sizes = [
    1, 2, 3, 4, 5, 6, 8, 10, 16, 32
];

pragma(inline, true)
chunk_kind granules_to_chunk_kind(size_t granules) {
    static foreach(gsize; small_object_granule_sizes) {
        if (granules <= gsize)
            return mixin(q{chunk_kind.GRANULES_}, cast(int)gsize);
    }
    return chunk_kind.LARGE_OBJECT;
}

pragma(inline, true)
ubyte chunk_kind_to_granules(chunk_kind kind) {
    static foreach(gsize; small_object_granule_sizes) {
        if (kind == mixin(q{chunk_kind.GRANULES_}, cast(int)gsize))
            return gsize;
    }
    return cast(ubyte)-1;
}

struct _page_header_t {
    ubyte[CHUNKS_PER_PAGE] chunk_kinds;
}

struct _page_t {
    union {
        _page_header_t header;
        _chunk_t[CHUNKS_PER_PAGE] chunks;
    }
}

pragma(inline, true)
_page_t* get_page(void *ptr) {
    return cast(_page_t*)cast(void*)((cast(size_t) ptr) & ~PAGE_MASK);
}

pragma(inline, true)
static size_t get_chunk_index(void *ptr) {
    return ((cast(size_t) ptr) & PAGE_MASK) / CHUNK_SIZE;
}

struct _freelist_t {
  _freelist_t *next;
}

struct _large_object_t {
  _large_object_t* next;
  size_t size;
}

pragma(inline, true)
void* get_large_object_payload(_large_object_t *obj) {
  return (cast(void*)obj) + LARGE_OBJECT_HEADER_SIZE;
}

pragma(inline, true)
_large_object_t* get_large_object(void *ptr) {
  return cast(_large_object_t*)(ptr - LARGE_OBJECT_HEADER_SIZE);
}

_page_t* allocate_pages(size_t payloadSize, size_t* allocated) {
    size_t needed = payloadSize + PAGE_HEADER_SIZE;
    size_t heap_size = llvm_wasm_memory_size(0) * PAGE_SIZE;
    size_t base = heap_size;
    size_t preallocated = 0, grow = 0;

    if (!walloc_heap_size) {
        // We are allocating the initial pages, if any.  We skip the first 64 kB,
        // then take any additional space up to the memory size.
        size_t heap_base = _alignv(cast(size_t)&__heap_base, PAGE_SIZE);
        preallocated = heap_size - heap_base; // Preallocated pages.
        walloc_heap_size = preallocated;
        base -= preallocated;
    }

    if (preallocated < needed) {
        // Always grow the walloc heap at least by 50%.
        grow = _alignv(_max(walloc_heap_size / 2, needed - preallocated),
                        PAGE_SIZE);

        assert(grow);
        if (llvm_wasm_memory_grow(0, cast(int)(grow >> PAGE_SIZE_LOG_2)) == -1) {
            return null;
        }

        walloc_heap_size += grow;
    }

    _page_t* ret = cast(_page_t*)base;
    size_t size = grow + preallocated;

    assert(size);
    assert(size == _alignv(size, PAGE_SIZE));
    *allocated = size / PAGE_SIZE;
    return ret;
}

void* allocate_chunk(_page_t* page, size_t idx, chunk_kind kind) {
    page.header.chunk_kinds[idx] = kind;
    return page.chunks[idx].data.ptr;
}

// It's possible for splitting to produce a large object of size 248 (256 minus
// the header size) -- i.e. spanning a single chunk.  In that case, push the
// chunk back on the GRANULES_32 small object freelist.
void maybe_repurpose_single_chunk_large_objects_head() {
    if (large_objects.size < CHUNK_SIZE) {
        size_t idx = get_chunk_index(large_objects);
        void* ptr = allocate_chunk(get_page(large_objects), idx, chunk_kind.GRANULES_32);
        large_objects = large_objects.next;
        _freelist_t* head = cast(_freelist_t*)ptr;
        head.next = small_object_freelists[chunk_kind.GRANULES_32];
        small_object_freelists[chunk_kind.GRANULES_32] = head;
    }
}

// If there have been any large-object frees since the last large object
// allocation, go through the freelist and merge any adjacent objects.
__gshared int pending_large_object_compact = 0;
_large_object_t** maybe_merge_free_large_object(_large_object_t** prev) {
    _large_object_t* obj = *prev;

    while(true) {
        void* end = get_large_object_payload(obj) + obj.size;
        __assert_aligned(end, CHUNK_SIZE);

        size_t chunk = get_chunk_index(end);
        if (chunk < FIRST_ALLOCATABLE_CHUNK) {
            // Merging can't create a large object that newly spans the header chunk.
            // This check also catches the end-of-heap case.
            return prev;
        }
        _page_t* page = get_page(end);
        if (page.header.chunk_kinds[chunk] != chunk_kind.FREE_LARGE_OBJECT) {
            return prev;
        }
        _large_object_t* next = cast(_large_object_t*)end;

        _large_object_t** prev_prev = &large_objects;
        _large_object_t* walk = large_objects;
        while(true) {
            assert(walk);
            if (walk == next) {
                obj.size += LARGE_OBJECT_HEADER_SIZE + walk.size;
                *prev_prev = walk.next;
                if (prev == &walk.next) {
                    prev = prev_prev;
                }
                break;
            }
            prev_prev = &walk.next;
            walk = walk.next;
        }
    }
}

void maybe_compact_free_large_objects() {
    if (pending_large_object_compact) {
        pending_large_object_compact = 0;
        _large_object_t** prev = &large_objects;
        while (*prev) {
            prev = &(*maybe_merge_free_large_object(prev)).next;
        }
    }
}

// Allocate a large object with enough space for SIZE payload bytes.  Returns a
// large object with a header, aligned on a chunk boundary, whose payload size
// may be larger than SIZE, and whose total size (header included) is
// chunk-aligned.  Either a suitable allocation is found in the large object
// freelist, or we ask the OS for some more pages and treat those pages as a
// large object.  If the allocation fits in that large object and there's more
// than an aligned chunk's worth of data free at the end, the large object is
// split.
//
// The return value's corresponding chunk in the page as starting a large
// object.
_large_object_t* allocate_large_object(size_t size) {
    maybe_compact_free_large_objects();

    _large_object_t* best = null;
    _large_object_t** best_prev = &large_objects;
    size_t best_size = -1;

    _large_object_t** prev = &large_objects;
    _large_object_t* walk = large_objects;
    while (walk) {
        if (walk.size >= size && walk.size < best_size) {
            best_size = walk.size;
            best = walk;
            best_prev = prev;

            // Not going to do any better than this; just return it.
            if (best_size + LARGE_OBJECT_HEADER_SIZE == _alignv(size + LARGE_OBJECT_HEADER_SIZE, CHUNK_SIZE))
                break;
        }

        prev = &walk.next;
        walk = walk.next;
    }

    if (!best) {
        // The large object freelist doesn't have an object big enough for this
        // allocation.  Allocate one or more pages from the OS, and treat that new
        // sequence of pages as a fresh large object.  It will be split if
        // necessary.
        size_t size_with_header = size + _large_object_t.sizeof;
        size_t n_allocated = 0;
        _page_t* page = allocate_pages(size_with_header, &n_allocated);
        if (!page) {
            return null;
        }

        void* ptr = allocate_chunk(page, FIRST_ALLOCATABLE_CHUNK, chunk_kind.LARGE_OBJECT);
        best = cast(_large_object_t*)ptr;
        size_t page_header = ptr - cast(void*)page;

        best.next = large_objects;
        best.size = best_size = n_allocated * PAGE_SIZE - page_header - LARGE_OBJECT_HEADER_SIZE;
        assert(best_size >= size_with_header);
    }

    allocate_chunk(get_page(best), get_chunk_index(best), chunk_kind.LARGE_OBJECT);

    _large_object_t* next = best.next;
    *best_prev = next;

    size_t tail_size = (best_size - size) & ~CHUNK_MASK;
    if (tail_size) {
        // The best-fitting object has 1 or more aligned chunks free after the
        // requested allocation; split the tail off into a fresh aligned object.
        _page_t* start_page = get_page(best);
        void* start = get_large_object_payload(best);
        void* end = start + best_size;

        if (start_page == get_page(end - tail_size - 1)) {

            // The allocation does not span a page boundary; yay.
            __assert_aligned(end, CHUNK_SIZE);
        } else if (size < PAGE_SIZE - LARGE_OBJECT_HEADER_SIZE - CHUNK_SIZE) {

            // If the allocation itself smaller than a page, split off the head, then
            // fall through to maybe split the tail.
            assert(cast(size_t)end == _alignv(cast(size_t)end, PAGE_SIZE));

            size_t first_page_size = PAGE_SIZE - (cast(size_t)start & PAGE_MASK);
            _large_object_t* head = best;
            allocate_chunk(start_page, get_chunk_index(start), chunk_kind.FREE_LARGE_OBJECT);
            head.size = first_page_size;
            head.next = large_objects;
            large_objects = head;

            maybe_repurpose_single_chunk_large_objects_head();

            _page_t* next_page = start_page + 1;
            void* ptr = allocate_chunk(next_page, FIRST_ALLOCATABLE_CHUNK, chunk_kind.LARGE_OBJECT);
            best = cast(_large_object_t*)ptr;
            best.size = best_size = best_size - first_page_size - CHUNK_SIZE - LARGE_OBJECT_HEADER_SIZE;
            assert(best_size >= size);

            start = get_large_object_payload(best);
            tail_size = (best_size - size) & ~CHUNK_MASK;
        } else {

            // A large object that spans more than one page will consume all of its
            // tail pages.  Therefore if the split traverses a page boundary, round up
            // to page size.
            __assert_aligned(end, PAGE_SIZE);
            size_t first_page_size = PAGE_SIZE - (cast(size_t)start & PAGE_MASK);
            size_t tail_pages_size = _alignv(size - first_page_size, PAGE_SIZE);
            size = first_page_size + tail_pages_size;
            tail_size = best_size - size;
        }
        best.size -= tail_size;

        size_t tail_idx = get_chunk_index(end - tail_size);
        while (tail_idx < FIRST_ALLOCATABLE_CHUNK && tail_size) {

            // We would be splitting in a page header; don't do that.
            tail_size -= CHUNK_SIZE;
            tail_idx++;
        }

        if (tail_size) {
            _page_t *page = get_page(end - tail_size);
            void* tail_ptr = allocate_chunk(page, tail_idx, chunk_kind.FREE_LARGE_OBJECT);
            _large_object_t* tail = cast(_large_object_t*) tail_ptr;
            tail.next = large_objects;
            tail.size = tail_size - LARGE_OBJECT_HEADER_SIZE;

            debug {
                size_t payloadsz = cast(size_t)get_large_object_payload(tail) + tail.size;
                assert(payloadsz == _alignv(payloadsz, CHUNK_SIZE));
            }

            large_objects = tail;
            maybe_repurpose_single_chunk_large_objects_head();
        }
    }

    debug {
        size_t payloadsz = cast(size_t)get_large_object_payload(best) + best.size;
        assert(payloadsz == _alignv(payloadsz, CHUNK_SIZE));
    }
    return best;
}

_freelist_t* obtain_small_objects(chunk_kind kind) {
    _freelist_t** whole_chunk_freelist = &small_object_freelists[chunk_kind.GRANULES_32];
    void *chunk;
    if (*whole_chunk_freelist) {
        chunk = *whole_chunk_freelist;
        *whole_chunk_freelist = (*whole_chunk_freelist).next;
    } else {
        chunk = allocate_large_object(0);
        if (!chunk) {
            return null;
        }
    }

    void* ptr = allocate_chunk(get_page(chunk), get_chunk_index(chunk), kind);
    void* end = ptr + CHUNK_SIZE;
    _freelist_t* next = null;
    size_t size = chunk_kind_to_granules(kind) * GRANULE_SIZE;
    for (size_t i = size; i <= CHUNK_SIZE; i += size) {
        _freelist_t* head = cast(_freelist_t*)(end - i);
        head.next = next;
        next = head;
    }
    return next;
}

pragma(inline, true)
size_t size_to_granules(size_t size) {
    return (size + GRANULE_SIZE - 1) >> GRANULE_SIZE_LOG_2;
}

pragma(inline, true)
_freelist_t** get_small_object_freelist(chunk_kind kind) {
    assert(kind < chunk_kind.SMALL_OBJECT_CHUNK_KINDS);
    return &small_object_freelists[kind];
}

void* allocate_small(chunk_kind kind) {
    _freelist_t** loc = get_small_object_freelist(kind);
    if (!*loc) {
        _freelist_t* freelist = obtain_small_objects(kind);
        if (!freelist)
            return null;

        *loc = freelist;
    }

    _freelist_t* ret = *loc;
    *loc = ret.next;
    return cast(void*)ret;
}

void* allocate_large(size_t size) {
  _large_object_t* obj = allocate_large_object(size);
  return obj ? get_large_object_payload(obj) : null;
}
