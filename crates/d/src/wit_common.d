module wit.common;

/// Thin CABI compliant wrapper over `T[]`
struct List(T) {
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
alias String = List!(immutable char);

// TODO: split this file up and give Tuple a full port of the Phobos version?
/// adapted from Phobos std.typecons.Tuple
/// No support for naming members.
struct Tuple(Types...) if (is(Types)) {
    Types expand;
    alias expand this;
}

/// adapted from Phobos std.bitmanip.BitFlags
struct Flags(Enum) if (is(Enum == enum)) {
@safe @nogc pure nothrow:
    public alias E = Enum;

private:
    template allAreBaseEnum(T...)
    {
        static foreach (Ti; T)
        {
            static if (!is(typeof(allAreBaseEnum) == bool) && // not yet defined
                    !is(Ti : E))
            {
                enum allAreBaseEnum = false;
            }
        }
        static if (!is(typeof(allAreBaseEnum) == bool)) // if not yet defined
        {
            enum allAreBaseEnum = true;
        }
    }

    static if (is(E U == enum)) {
        alias Base = U;
    } else static assert(0);

    Base mValue;

public:
    this(E flag)
    {
        this = flag;
    }

    this(T...)(T flags)
    if (allAreBaseEnum!(T))
    {
        this = flags;
    }

    bool opCast(B: bool)() const
    {
        return mValue != 0;
    }

    Base opCast(B)() const
    if (is(Base : B))
    {
        return mValue;
    }

    auto opUnary(string op)() const
    if (op == "~")
    {
        return WitFlags(cast(E) cast(Base) ~mValue);
    }

    auto ref opAssign(T...)(T flags)
    if (allAreBaseEnum!(T))
    {
        mValue = 0;
        foreach (E flag; flags)
        {
            mValue |= flag;
        }
        return this;
    }

    auto ref opAssign(E flag)
    {
        mValue = flag;
        return this;
    }

    auto ref opOpAssign(string op: "|")(WitFlags flags)
    {
        mValue |= flags.mValue;
        return this;
    }

    auto ref opOpAssign(string op: "&")(WitFlags  flags)
    {
        mValue &= flags.mValue;
        return this;
    }

    auto ref opOpAssign(string op: "|")(E flag)
    {
        mValue |= flag;
        return this;
    }

    auto ref opOpAssign(string op: "&")(E flag)
    {
        mValue &= flag;
        return this;
    }

    auto opBinary(string op)(WitFlags flags) const
    if (op == "|" || op == "&")
    {
        WitFlags result = this;
        result.opOpAssign!op(flags);
        return result;
    }

    auto opBinary(string op)(E flag) const
    if (op == "|" || op == "&")
    {
        WitFlags result = this;
        result.opOpAssign!op(flag);
        return result;
    }

    auto opBinaryRight(string op)(E flag) const
    if (op == "|" || op == "&")
    {
        return opBinary!op(flag);
    }

    bool opDispatch(string name)() const
    if (__traits(hasMember, E, name))
    {
        enum e = __traits(getMember, E, name);
        return (mValue & e) == e;
    }

    void opDispatch(string name)(bool set)
    if (__traits(hasMember, E, name))
    {
        enum e = __traits(getMember, E, name);
        if (set)
            mValue |= e;
        else
            mValue &= ~e;
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
