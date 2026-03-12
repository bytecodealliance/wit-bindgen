module wit.common;

/// Thin CABI compliant wrapper over `T[]`
struct WitList(T) {
@safe @nogc pure nothrow:
    T* ptr;
    size_t length;

    this(T[] slice) @trusted {
        this = slice;
    }

    void opAssign(T[] slice) @trusted {
        ptr = slice.ptr;
        length = slice.length;
    }

    alias asSlice this;
    inout(T)[] asSlice() @trusted inout {
        return (ptr && length) ? ptr[0..length] : null;
    }
}

// WIT ABI for string matches List,
// except list<char> in WIT is actually List!(dchar)
//
// We assume UTF-8 data (as D native strings are UTF-8)
alias WitString = List!(char);

// TODO: split this file up and give Tuple a full port of the Phobos version?
/// adapted from Phobos std.typecons.Tuple
/// No support for naming members.
struct Tuple(Types...) if (is(Types)) {
    Types expand;
    alias expand this;
}

mixin template WitFlags(T) if (__traits(isUnsigned, T)) {
    private alias F = typeof(this);

    T bits;

    @safe nothrow @nogc pure:

    static typeof(this) opIndex(size_t i)
    in(i < T.sizeof*8) => F(cast(T)(1 << i));

    auto opUnary(string op : "~")() const => F(~bits);

    auto ref opOpAssign(string op)(F rhs)
    if (op == "|" || op == "&" || op == "^")
    {
        mixin("bits "~op~"= rhs.bits;");
        return this;
    }

    auto opBinary(string op)(F flags) const
    if (op == "|" || op == "&" || op == "^")
    {
        F result = this;
        result.opOpAssign!op(flags);
        return result;
    }
}

/// Based on Rust's Option
struct Option(T) {
private:
    bool present = false;
    T value;

    @disable this();
    this(bool present, T value = T.init) @safe @nogc nothrow {
        this.present = present;
        this.value = value;
    }
public:
    static Option some(T value) @safe @nogc nothrow {
        return Option(true, value);
    }

    static Option none() @safe @nogc nothrow {
        return Option(false);
    }

    bool isSome() const @safe @nogc nothrow => present;
    alias isSome this; // implicit conversion to bool

    bool isNone() const @safe @nogc nothrow => !present;

    ref inout(T) unwrap() inout @safe @nogc nothrow return
    in (present) do { return value; }

    T unwrapOr(T fallback) @safe @nogc nothrow => present ? value : fallback;

    T unwrapOrElse(D)(scope D fallback)
    if (is(D R == return) && is(R : T) && is(D == __parameters))
    { return present ? value : fallback(); }
}

/// Based on Rust's Result
struct Result(T, E) {
private:
    bool hasError;
    union Storage {
        ubyte __zeroinit = 0;
        static if (!is(T == void)) {
            T value;
        }
        static if (!is(E == void)) {
            E error;
        }
    }
    Storage storage;

    @disable this();
    this(bool hasError, Storage storage) @safe @nogc nothrow {
        this.hasError = hasError;
        this.storage = storage;
    }

public:
    static if (is(T == void)) {
        static Result ok() @safe @nogc nothrow => Result(false, Storage());
    } else {
        static Result ok(T value) @safe @nogc nothrow {
            Storage newStorage;
            newStorage.value = value;

            return Result(false, newStorage);
        }
    }

    static if (is(E == void)) {
        static Result err() @safe @nogc nothrow => Result(true, Storage());
    } else {
        static Result err(E error) @safe @nogc nothrow {
            Storage newStorage;
            newStorage.error = error;

            return Result(true, newStorage);
        }
    }

    bool isOk() const @safe @nogc nothrow => !hasError;

    bool isErr() const @safe @nogc nothrow => hasError;
    alias isErr this; // implicit conversion to bool

    static if (!is(T == void)) {
        ref inout(T) unwrap() inout @safe @nogc nothrow return
        in (isOk) do { return storage.value; }

        T unwrapOr(T fallback) @safe @nogc nothrow => isOk ? storage.value : fallback;

        T unwrapOrElse(D)(scope D fallback)
        if (is(D R == return) && is(R : T) && is(D == __parameters))
        { return isOk ? storage.value : fallback(); }
    }

    static if (!is(E == void)) {
        ref inout(E) unwrapErr() inout @safe @nogc nothrow return
        in (isErr) do { return storage.error; }
    }
}
