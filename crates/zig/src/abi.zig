//! Component model canonical ABI, driven by Zig types at comptime.
//!
//! WIT types map to Zig like this:
//!   string, list<T>   []const u8, []const T
//!   record, tuple     struct, tuple
//!   variant, result   union(enum)
//!   option<T>         ?T
//!   enum              enum(uN)
//!   flags             packed struct(uN) of bools
//!   own<R>            non-exhaustive enum(u32)
//!   borrow<R>         Borrowed(R), parameters only

const std = @import("std");
const builtin = @import("builtin");

pub fn Result(comptime T: type, comptime E: type) type {
    return union(enum) { ok: T, err: E };
}

const is_wasm = builtin.cpu.arch == .wasm32;
const backing = if (is_wasm) std.heap.wasm_allocator else std.heap.page_allocator;

/// For memory that outlives the current call.
pub const heap = backing;

var call_arena: std.heap.ArenaAllocator = .init(backing);

/// Everything the host hands us lives here and is gone once the current export returns.
pub fn arena() std.mem.Allocator {
    return call_arena.allocator();
}

const DropFn = *const fn (i32) callconv(.c) void;
const Tracked = struct { handle: u32, drop: DropFn };

/// Owned handles received during the current export, newest last.
var tracked: std.ArrayList(Tracked) = .empty;

fn track(comptime T: type, handle: T) void {
    tracked.append(backing, .{ .handle = @intFromEnum(handle), .drop = T.resource_drop }) catch @panic("OOM");
}

fn untrack(comptime T: type, handle: T) void {
    var i = tracked.items.len;
    while (i > 0) {
        i -= 1;
        const item = tracked.items[i];
        if (item.handle == @intFromEnum(handle) and item.drop == T.resource_drop) {
            _ = tracked.orderedRemove(i);
            for (frames.items) |*start| {
                if (start.* > i) start.* -= 1;
            }
            return;
        }
    }
}

/// Takes a handle out of the per-call cleanup so it can be stored across calls.
/// From then on it has to be released with `deinit`.
pub fn keep(comptime T: type, handle: T) void {
    untrack(T, handle);
}

pub fn drop(comptime T: type, handle: T) void {
    untrack(T, handle);
    T.resource_drop(@bitCast(@intFromEnum(handle)));
}

/// A handle that is only lent to the callee. Plain handles are given away when lowered.
pub fn Borrowed(comptime T: type) type {
    return struct {
        handle: T,

        pub const wit_borrowed = {};
    };
}

pub fn borrow(handle: anytype) Borrowed(@TypeOf(handle)) {
    return .{ .handle = handle };
}

fn isBorrowed(comptime T: type) bool {
    return @typeInfo(T) == .@"struct" and @hasDecl(T, "wit_borrowed");
}

pub const Stats = struct {
    /// Size of the linear memory. Wasm memory never shrinks, so this is the
    /// number to watch for leaks.
    memory_bytes: usize,
    /// What the per-call arena holds on to between calls.
    arena_bytes: usize,
    /// Handles that will be released when the current call ends.
    tracked_handles: usize,
};

pub fn stats() Stats {
    return .{
        .memory_bytes = if (is_wasm) @wasmMemorySize(0) * std.wasm.page_size else 0,
        .arena_bytes = call_arena.queryCapacity(),
        .tracked_handles = tracked.items.len,
    };
}

/// Where each running export starts in `tracked`. Exports nest when a host
/// call runs one of ours before returning.
var frames: std.ArrayList(usize) = .empty;

pub fn beginCall() void {
    frames.append(backing, tracked.items.len) catch @panic("OOM");
}

/// Drops the handles nobody kept. This calls into the host, which is not
/// allowed from a post-return function, so it can't be left to `endCall`.
pub fn releaseHandles() void {
    const start = if (frames.items.len > 0) frames.items[frames.items.len - 1] else 0;
    while (tracked.items.len > start) {
        const item = tracked.pop().?;
        item.drop(@bitCast(item.handle));
    }
}

/// Resets the arena once the outermost export is done. With an indirect result
/// the host still reads from it after the export returns, those exports call
/// this from `cabi_post_*`.
pub fn endCall() void {
    releaseHandles();
    _ = frames.pop();
    if (frames.items.len == 0) _ = call_arena.reset(.retain_capacity);
}

fn cabiRealloc(old: ?[*]u8, old_len: usize, alignment: usize, new_len: usize) callconv(.c) ?[*]u8 {
    const a: std.mem.Alignment = .fromByteUnits(alignment);
    const new = arena().rawAlloc(new_len, a, @returnAddress()) orelse @panic("OOM");
    if (old) |p| @memcpy(new[0..old_len], p[0..old_len]);
    return new;
}

comptime {
    if (is_wasm) @export(&cabiRealloc, .{ .name = "cabi_realloc" });
}

const Core = enum { i32, i64, f32, f64 };

fn CoreType(comptime c: Core) type {
    return switch (c) {
        .i32 => i32,
        .i64 => i64,
        .f32 => f32,
        .f64 => f64,
    };
}

const max_flat_params = 16;

// The event variant alone has a few hundred cases.
const branch_quota = 10_000_000;

fn isHandle(comptime T: type) bool {
    return @typeInfo(T) == .@"enum" and !@typeInfo(T).@"enum".is_exhaustive;
}

fn Discriminant(comptime cases: usize) type {
    if (cases <= 1 << 8) return u8;
    if (cases <= 1 << 16) return u16;
    return u32;
}

fn caseCount(comptime T: type) comptime_int {
    return switch (@typeInfo(T)) {
        .optional => 2,
        .@"union" => |u| u.fields.len,
        .@"enum" => |e| e.fields.len,
        else => unreachable,
    };
}

/// Whether a []const T already has the canonical layout in memory.
fn isPlain(comptime T: type) bool {
    return switch (@typeInfo(T)) {
        .int => |i| i.bits == 8 or i.bits == 16 or i.bits == 32 or i.bits == 64,
        .float => true,
        else => false,
    };
}

pub fn alignOf(comptime T: type) comptime_int {
    @setEvalBranchQuota(branch_quota);
    return switch (@typeInfo(T)) {
        .void => 1,
        .bool => 1,
        .int => |i| if (i.bits == 21) 4 else @divExact(i.bits, 8),
        .float => |f| @divExact(f.bits, 8),
        .pointer => 4,
        .@"enum" => if (isHandle(T)) 4 else @sizeOf(Discriminant(caseCount(T))),
        .optional => |o| @max(1, alignOf(o.child)),
        .@"union" => |u| blk: {
            var a = @sizeOf(Discriminant(u.fields.len));
            for (u.fields) |f| a = @max(a, alignOf(f.type));
            break :blk a;
        },
        .@"struct" => |s| blk: {
            if (s.layout == .@"packed") break :blk @sizeOf(s.backing_integer.?);
            if (isBorrowed(T)) break :blk 4;
            var a = 1;
            for (s.fields) |f| a = @max(a, alignOf(f.type));
            break :blk a;
        },
        else => @compileError("no canonical ABI for " ++ @typeName(T)),
    };
}

fn alignUp(comptime n: comptime_int, comptime a: comptime_int) comptime_int {
    return @divFloor(n + a - 1, a) * a;
}

pub fn sizeOf(comptime T: type) comptime_int {
    @setEvalBranchQuota(branch_quota);
    return switch (@typeInfo(T)) {
        .void => 0,
        .bool => 1,
        .int => |i| if (i.bits == 21) 4 else @divExact(i.bits, 8),
        .float => |f| @divExact(f.bits, 8),
        .pointer => 8,
        .@"enum" => if (isHandle(T)) 4 else @sizeOf(Discriminant(caseCount(T))),
        .optional => |o| alignUp(payloadOffset(T) + sizeOf(o.child), alignOf(T)),
        .@"union" => |u| blk: {
            var max = 0;
            for (u.fields) |f| max = @max(max, sizeOf(f.type));
            break :blk alignUp(payloadOffset(T) + max, alignOf(T));
        },
        .@"struct" => |s| blk: {
            if (s.layout == .@"packed") break :blk @sizeOf(s.backing_integer.?);
            if (isBorrowed(T)) break :blk 4;
            var n = 0;
            for (s.fields) |f| n = alignUp(n, alignOf(f.type)) + sizeOf(f.type);
            break :blk alignUp(n, alignOf(T));
        },
        else => @compileError("no canonical ABI for " ++ @typeName(T)),
    };
}

fn payloadOffset(comptime T: type) comptime_int {
    @setEvalBranchQuota(branch_quota);
    const disc = @sizeOf(Discriminant(caseCount(T)));
    const payload_align = switch (@typeInfo(T)) {
        .optional => |o| alignOf(o.child),
        .@"union" => alignOf(T),
        else => unreachable,
    };
    return alignUp(disc, payload_align);
}

fn fieldOffset(comptime T: type, comptime index: usize) comptime_int {
    @setEvalBranchQuota(branch_quota);
    var n = 0;
    for (@typeInfo(T).@"struct".fields, 0..) |f, i| {
        n = alignUp(n, alignOf(f.type));
        if (i == index) return n;
        n += sizeOf(f.type);
    }
    unreachable;
}

fn join(a: Core, b: Core) Core {
    if (a == b) return a;
    if ((a == .i32 and b == .f32) or (a == .f32 and b == .i32)) return .i32;
    return .i64;
}

fn flattenCases(comptime payloads: []const type) []const Core {
    var out: []const Core = &.{};
    for (payloads) |P| {
        const flat = flatten(P);
        var merged: []const Core = &.{};
        for (0..@max(out.len, flat.len)) |i| {
            const c = if (i >= out.len) flat[i] else if (i >= flat.len) out[i] else join(out[i], flat[i]);
            merged = merged ++ [1]Core{c};
        }
        out = merged;
    }
    return [1]Core{.i32} ++ out;
}

pub fn flatten(comptime T: type) []const Core {
    return comptime blk: {
        @setEvalBranchQuota(branch_quota);
        break :blk flattenType(T);
    };
}

fn flattenType(comptime T: type) []const Core {
    return switch (@typeInfo(T)) {
        .void => &.{},
        .bool, .@"enum" => &.{.i32},
        .int => |i| if (i.bits == 64) &.{.i64} else &.{.i32},
        .float => |f| if (f.bits == 64) &.{.f64} else &.{.f32},
        .pointer => &.{ .i32, .i32 },
        .optional => |o| flattenCases(&.{ void, o.child }),
        .@"union" => |u| blk: {
            var payloads: []const type = &.{};
            for (u.fields) |f| payloads = payloads ++ [1]type{f.type};
            break :blk flattenCases(payloads);
        },
        .@"struct" => |s| blk: {
            if (s.layout == .@"packed" or isBorrowed(T)) break :blk &.{.i32};
            var out: []const Core = &.{};
            for (s.fields) |f| out = out ++ flatten(f.type);
            break :blk out;
        },
        else => @compileError("no canonical ABI for " ++ @typeName(T)),
    };
}

// Flat values travel as raw bits in u64 slots, which makes the variant
// payload joins (f32 in an i32 slot, i32 in an i64 slot, ...) free.

fn lowerSlice(comptime T: type, items: []const T, slots: []u64) void {
    slots[1] = items.len;
    if (items.len == 0) {
        slots[0] = 0;
        return;
    }
    if (comptime isPlain(T)) {
        slots[0] = @intFromPtr(items.ptr);
        return;
    }
    const a: std.mem.Alignment = comptime .fromByteUnits(alignOf(T));
    const buf = arena().alignedAlloc(u8, a, items.len * sizeOf(T)) catch @panic("OOM");
    for (items, 0..) |item, i| store(T, item, buf.ptr + i * sizeOf(T));
    slots[0] = @intFromPtr(buf.ptr);
}

fn liftSlice(comptime T: type, addr: u64, len: u64) []const T {
    if (len == 0) return &.{};
    const n: usize = @intCast(len);
    if (comptime isPlain(T)) {
        const ptr: [*]const T = @ptrFromInt(@as(usize, @intCast(addr)));
        return ptr[0..n];
    }
    const base: [*]const u8 = @ptrFromInt(@as(usize, @intCast(addr)));
    const out = arena().alloc(T, n) catch @panic("OOM");
    for (out, 0..) |*item, i| item.* = load(T, base + i * sizeOf(T));
    return out;
}

pub fn lowerFlat(comptime T: type, value: T, slots: []u64) void {
    switch (@typeInfo(T)) {
        .void => {},
        .bool => slots[0] = @intFromBool(value),
        .int => |i| {
            if (i.bits == 64) {
                slots[0] = @bitCast(value);
            } else if (i.signedness == .signed) {
                slots[0] = @as(u32, @bitCast(@as(i32, value)));
            } else {
                slots[0] = value;
            }
        },
        .float => |f| slots[0] = if (f.bits == 64) @bitCast(value) else @as(u32, @bitCast(value)),
        .pointer => |p| lowerSlice(p.child, value, slots),
        .@"enum" => {
            if (comptime isHandle(T)) untrack(T, value);
            slots[0] = @intFromEnum(value);
        },
        .optional => |o| {
            @memset(slots[0..comptime flatten(T).len], 0);
            if (value) |v| {
                slots[0] = 1;
                lowerFlat(o.child, v, slots[1..]);
            }
        },
        .@"union" => |u| {
            @memset(slots[0..comptime flatten(T).len], 0);
            switch (value) {
                inline else => |v, tag| {
                    slots[0] = @intFromEnum(tag);
                    lowerFlat(u.fields[@intFromEnum(tag)].type, v, slots[1..]);
                },
            }
        },
        .@"struct" => |s| {
            if (s.layout == .@"packed") {
                slots[0] = @as(s.backing_integer.?, @bitCast(value));
                return;
            }
            if (comptime isBorrowed(T)) {
                slots[0] = @intFromEnum(value.handle);
                return;
            }
            var at: usize = 0;
            inline for (s.fields) |f| {
                lowerFlat(f.type, @field(value, f.name), slots[at..]);
                at += comptime flatten(f.type).len;
            }
        },
        else => comptime unreachable,
    }
}

pub fn liftFlat(comptime T: type, slots: []const u64) T {
    switch (@typeInfo(T)) {
        .void => return {},
        .bool => return slots[0] != 0,
        .int => |i| {
            if (i.bits == 64) return @bitCast(slots[0]);
            const word: u32 = @truncate(slots[0]);
            if (i.signedness == .signed) return @truncate(@as(i32, @bitCast(word)));
            return @truncate(word);
        },
        .float => |f| {
            if (f.bits == 64) return @bitCast(slots[0]);
            return @bitCast(@as(u32, @truncate(slots[0])));
        },
        .pointer => |p| return liftSlice(p.child, slots[0], slots[1]),
        .@"enum" => {
            const value: T = @enumFromInt(slots[0]);
            if (comptime isHandle(T)) track(T, value);
            return value;
        },
        .optional => |o| return if (slots[0] == 0) null else liftFlat(o.child, slots[1..]),
        .@"union" => |u| {
            inline for (u.fields, 0..) |f, i| {
                if (slots[0] == i) return @unionInit(T, f.name, liftFlat(f.type, slots[1..]));
            }
            unreachable;
        },
        .@"struct" => |s| {
            if (s.layout == .@"packed") return @bitCast(@as(s.backing_integer.?, @truncate(slots[0])));
            if (comptime isBorrowed(T)) @compileError("borrowed handles can only be passed to imports");
            var out: T = undefined;
            var at: usize = 0;
            inline for (s.fields) |f| {
                @field(out, f.name) = liftFlat(f.type, slots[at..]);
                at += comptime flatten(f.type).len;
            }
            return out;
        },
        else => comptime unreachable,
    }
}

// A plugin only ever looks at a few cases of the big variants (there are
// hundreds of events). These let it lift and lower one case without pulling
// in code for all the others.

pub fn loadTag(comptime U: type, ptr: [*]const u8) std.meta.Tag(U) {
    return @enumFromInt(get(Discriminant(caseCount(U)), ptr));
}

pub fn loadCase(comptime U: type, comptime tag: std.meta.Tag(U), ptr: [*]const u8) @FieldType(U, @tagName(tag)) {
    return load(@FieldType(U, @tagName(tag)), ptr + payloadOffset(U));
}

pub fn storeCase(comptime U: type, comptime tag: std.meta.Tag(U), value: @FieldType(U, @tagName(tag)), ptr: [*]u8) void {
    put(Discriminant(caseCount(U)), ptr, @intFromEnum(tag));
    store(@FieldType(U, @tagName(tag)), value, ptr + payloadOffset(U));
}

/// Offset of field `index` inside the record `T` in linear memory.
pub fn offsetOf(comptime T: type, comptime index: usize) comptime_int {
    return fieldOffset(T, index);
}

/// Whether parameters of this shape arrive as a pointer instead of flat values.
pub fn passedInMemory(comptime Args: type) bool {
    return flatten(Args).len > max_flat_params;
}

pub fn alloc(comptime T: type) [*]u8 {
    return scratch(T);
}

fn put(comptime Int: type, ptr: [*]u8, value: Int) void {
    std.mem.writeInt(Int, ptr[0..@sizeOf(Int)], value, .little);
}

fn get(comptime Int: type, ptr: [*]const u8) Int {
    return std.mem.readInt(Int, ptr[0..@sizeOf(Int)], .little);
}

pub fn store(comptime T: type, value: T, ptr: [*]u8) void {
    switch (@typeInfo(T)) {
        .void => {},
        .bool => ptr[0] = @intFromBool(value),
        .int => |i| if (i.bits == 21) put(u32, ptr, value) else put(T, ptr, value),
        .float => |f| put(std.meta.Int(.unsigned, f.bits), ptr, @bitCast(value)),
        .pointer => |p| {
            var slots: [2]u64 = undefined;
            lowerSlice(p.child, value, &slots);
            put(u32, ptr, @intCast(slots[0]));
            put(u32, ptr + 4, @intCast(slots[1]));
        },
        .@"enum" => {
            if (comptime isHandle(T)) {
                untrack(T, value);
                return put(u32, ptr, @intFromEnum(value));
            }
            put(Discriminant(caseCount(T)), ptr, @intFromEnum(value));
        },
        .optional => |o| {
            ptr[0] = @intFromBool(value != null);
            if (value) |v| store(o.child, v, ptr + payloadOffset(T));
        },
        .@"union" => |u| switch (value) {
            inline else => |v, tag| {
                put(Discriminant(u.fields.len), ptr, @intFromEnum(tag));
                store(u.fields[@intFromEnum(tag)].type, v, ptr + payloadOffset(T));
            },
        },
        .@"struct" => |s| {
            if (s.layout == .@"packed") return put(s.backing_integer.?, ptr, @bitCast(value));
            if (comptime isBorrowed(T)) return put(u32, ptr, @intFromEnum(value.handle));
            inline for (s.fields, 0..) |f, i| {
                store(f.type, @field(value, f.name), ptr + fieldOffset(T, i));
            }
        },
        else => comptime unreachable,
    }
}

pub fn load(comptime T: type, ptr: [*]const u8) T {
    switch (@typeInfo(T)) {
        .void => return {},
        .bool => return ptr[0] != 0,
        .int => |i| return if (i.bits == 21) @truncate(get(u32, ptr)) else get(T, ptr),
        .float => |f| return @bitCast(get(std.meta.Int(.unsigned, f.bits), ptr)),
        .pointer => |p| return liftSlice(p.child, get(u32, ptr), get(u32, ptr + 4)),
        .@"enum" => {
            if (comptime isHandle(T)) {
                const value: T = @enumFromInt(get(u32, ptr));
                track(T, value);
                return value;
            }
            return @enumFromInt(get(Discriminant(caseCount(T)), ptr));
        },
        .optional => |o| return if (ptr[0] == 0) null else load(o.child, ptr + payloadOffset(T)),
        .@"union" => |u| {
            const tag = get(Discriminant(u.fields.len), ptr);
            inline for (u.fields, 0..) |f, i| {
                if (tag == i) return @unionInit(T, f.name, load(f.type, ptr + payloadOffset(T)));
            }
            unreachable;
        },
        .@"struct" => |s| {
            if (s.layout == .@"packed") return @bitCast(get(s.backing_integer.?, ptr));
            var out: T = undefined;
            inline for (s.fields, 0..) |f, i| {
                @field(out, f.name) = load(f.type, ptr + fieldOffset(T, i));
            }
            return out;
        },
        else => comptime unreachable,
    }
}

fn toCore(comptime C: type, slot: u64) C {
    return switch (C) {
        i32 => @bitCast(@as(u32, @truncate(slot))),
        i64 => @bitCast(slot),
        f32 => @bitCast(@as(u32, @truncate(slot))),
        f64 => @bitCast(slot),
        else => comptime unreachable,
    };
}

fn fromCore(value: anytype) u64 {
    return switch (@TypeOf(value)) {
        i32, f32 => @as(u32, @bitCast(value)),
        i64, f64 => @bitCast(value),
        else => comptime unreachable,
    };
}

fn scratch(comptime T: type) [*]u8 {
    const a: std.mem.Alignment = comptime .fromByteUnits(alignOf(T));
    const buf = arena().alignedAlloc(u8, a, @max(sizeOf(T), 1)) catch @panic("OOM");
    return buf.ptr;
}

/// The generator spells out raw signatures on its own, make sure it agrees with us.
fn checkSignature(comptime Func: type, comptime Args: type, comptime Ret: type) void {
    const info = @typeInfo(Func).@"fn";
    const args = flatten(Args);
    const ret = flatten(Ret);

    var expected: []const Core = if (args.len > max_flat_params) &.{.i32} else args;
    if (ret.len > 1) expected = expected ++ [1]Core{.i32};
    const expected_ret = if (ret.len == 1) CoreType(ret[0]) else void;

    var ok = info.params.len == expected.len and info.return_type.? == expected_ret;
    if (ok) for (info.params, expected) |p, e| {
        if (p.type.? != CoreType(e)) ok = false;
    };
    if (!ok) @compileError("raw signature " ++ @typeName(Func) ++ " does not match " ++ @typeName(Args) ++ " -> " ++ @typeName(Ret));
}

/// Calls an imported function. `func` is the raw extern with the flattened signature.
pub fn call(func: anytype, comptime Ret: type, args: anytype) Ret {
    const Args = @TypeOf(args);
    const Raw = std.meta.ArgsTuple(@TypeOf(func));
    const n = comptime flatten(Args).len;
    const ret_flat = comptime flatten(Ret).len;

    var slots: [@max(n, 1)]u64 = undefined;
    var raw: Raw = undefined;
    const used = comptime if (n > max_flat_params) 1 else n;
    comptime checkSignature(@TypeOf(func), Args, Ret);
    if (n > max_flat_params) {
        const buf = scratch(Args);
        store(Args, args, buf);
        raw[0] = @bitCast(@as(u32, @intFromPtr(buf)));
    } else {
        lowerFlat(Args, args, &slots);
        inline for (0..n) |i| raw[i] = toCore(@TypeOf(raw[i]), slots[i]);
    }

    if (ret_flat > 1) {
        const out = scratch(Ret);
        raw[used] = @bitCast(@as(u32, @intFromPtr(out)));
        @call(.auto, func, raw);
        return load(Ret, out);
    }
    if (ret_flat == 0) {
        @call(.auto, func, raw);
        return liftFlat(Ret, &.{});
    }
    const slot = fromCore(@call(.auto, func, raw));
    return liftFlat(Ret, &.{slot});
}

/// Turns the raw parameters of an export back into `Args`.
pub fn liftArgs(comptime Args: type, raw: anytype) Args {
    const n = comptime flatten(Args).len;
    if (n > max_flat_params) {
        const ptr: [*]const u8 = @ptrFromInt(@as(u32, @bitCast(raw[0])));
        return load(Args, ptr);
    }
    var slots: [@max(n, 1)]u64 = undefined;
    inline for (0..n) |i| slots[i] = fromCore(raw[i]);
    return liftFlat(Args, &slots);
}

pub fn RawReturn(comptime Ret: type) type {
    const flat = flatten(Ret);
    if (flat.len == 0) return void;
    if (flat.len == 1) return CoreType(flat[0]);
    return i32;
}

/// Lowers the return value of an export, the last thing an export does.
/// Handles that are part of `value` go back to the host, the rest is released.
pub fn lowerReturn(comptime Ret: type, value: Ret) RawReturn(Ret) {
    defer releaseHandles();
    const flat = comptime flatten(Ret);
    if (flat.len == 0) return;
    if (flat.len == 1) {
        var slots: [1]u64 = undefined;
        lowerFlat(Ret, value, &slots);
        return toCore(CoreType(flat[0]), slots[0]);
    }
    const out = scratch(Ret);
    store(Ret, value, out);
    return @bitCast(@as(u32, @intFromPtr(out)));
}

test "layout" {
    const Pos = struct { f64, f64, f64 };
    try std.testing.expectEqual(24, sizeOf(Pos));
    try std.testing.expectEqual(8, alignOf(Pos));

    const Rec = struct { a: u8, b: u32, c: []const u8, d: ?u16 };
    try std.testing.expectEqual(4, alignOf(Rec));
    try std.testing.expectEqual(20, sizeOf(Rec));

    const Var = union(enum) { a, b: u64, c: []const u8 };
    try std.testing.expectEqual(8, alignOf(Var));
    try std.testing.expectEqual(16, sizeOf(Var));
    try std.testing.expectEqual(2, sizeOf(?u8));
    try std.testing.expectEqual(16, sizeOf(?f64));
}

test "flatten" {
    try std.testing.expectEqualSlices(Core, &.{ .i32, .i32 }, flatten([]const u8));
    try std.testing.expectEqualSlices(Core, &.{ .i32, .f64 }, flatten(?f64));
    try std.testing.expectEqualSlices(Core, &.{ .i32, .i32 }, flatten(Result(f32, u32)));
    try std.testing.expectEqualSlices(Core, &.{ .i32, .i64, .i32 }, flatten(union(enum) { a: f64, b: []const u8, c }));
}

test "flat round trip" {
    const Rec = struct { a: i8, b: ?f32, c: Result(void, u64), d: packed struct(u8) { x: bool, y: bool, _: u6 = 0 } };
    const value: Rec = .{ .a = -3, .b = 1.5, .c = .{ .err = 1 << 40 }, .d = .{ .x = false, .y = true } };
    var slots: [flatten(Rec).len]u64 = undefined;
    lowerFlat(Rec, value, &slots);
    try std.testing.expectEqual(@as(u64, 0xffff_fffd), slots[0]);
    try std.testing.expectEqualDeep(value, liftFlat(Rec, &slots));
}

test "memory round trip" {
    const Rec = struct { a: u16, b: ?i64, c: union(enum) { x: f32, y: bool }, d: enum(u8) { p, q } };
    const value: Rec = .{ .a = 7, .b = -9, .c = .{ .y = true }, .d = .q };
    var buf: [sizeOf(Rec)]u8 align(8) = undefined;
    store(Rec, value, &buf);
    try std.testing.expectEqualDeep(value, load(Rec, &buf));
}

test "handle tracking" {
    const Res = enum(u32) {
        _,

        var dropped: [8]u32 = undefined;
        var count: usize = 0;

        pub const resource_drop = &record;

        fn record(handle: i32) callconv(.c) void {
            dropped[count] = @intCast(handle);
            count += 1;
        }
    };
    const Pair = struct { a: Res, b: ?Res };

    var slots: [flatten(Pair).len]u64 = .{ 1, 1, 2 };
    const pair = liftFlat(Pair, &slots);
    const third = liftFlat(Res, &.{3});
    const fourth = liftFlat(Res, &.{4});
    try std.testing.expectEqual(4, tracked.items.len);

    // Given away, lent, kept, dropped by hand. Only `a` is left for endCall.
    lowerFlat(Res, pair.b.?, &slots);
    lowerFlat(Borrowed(Res), borrow(pair.a), &slots);
    keep(Res, third);
    drop(Res, fourth);
    try std.testing.expectEqualSlices(u32, &.{4}, Res.dropped[0..Res.count]);

    lowerReturn(void, {});
    try std.testing.expectEqualSlices(u32, &.{ 4, 1 }, Res.dropped[0..Res.count]);
    try std.testing.expectEqual(0, tracked.items.len);
}

test "nested exports" {
    const Res = enum(u32) {
        _,

        var dropped: [8]u32 = undefined;
        var count: usize = 0;

        pub const resource_drop = &record;

        fn record(handle: i32) callconv(.c) void {
            dropped[count] = @intCast(handle);
            count += 1;
        }
    };

    beginCall();
    const outer = liftFlat(Res, &.{1});
    const text = try arena().dupe(u8, "still here");

    beginCall();
    _ = liftFlat(Res, &.{2});
    keep(Res, outer);
    _ = liftFlat(Res, &.{3});
    lowerReturn(void, {});
    endCall();
    try std.testing.expectEqualSlices(u32, &.{ 3, 2 }, Res.dropped[0..Res.count]);
    try std.testing.expectEqualStrings("still here", text);

    _ = liftFlat(Res, &.{4});
    lowerReturn(void, {});
    endCall();
    try std.testing.expectEqualSlices(u32, &.{ 3, 2, 4 }, Res.dropped[0..Res.count]);
    try std.testing.expectEqual(0, frames.items.len);
}
