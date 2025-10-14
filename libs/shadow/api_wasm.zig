const std = @import("std");
const Shadow = @import("src/shadow.zig").Shadow;

// Custom allocator that calls Emscripten's malloc/free directly
// This avoids linking libc which would cause errno conflicts between WASI and Emscripten
extern fn malloc(size: usize) ?*anyopaque;
extern fn free(ptr: *anyopaque) void;
extern fn realloc(ptr: ?*anyopaque, size: usize) ?*anyopaque;

fn emAlloc(ctx: *anyopaque, len: usize, ptr_align: std.mem.Alignment, ret_addr: usize) ?[*]u8 {
    _ = ctx;
    _ = ptr_align;
    _ = ret_addr;
    if (malloc(len)) |ptr| {
        return @ptrCast(ptr);
    }
    return null;
}

fn emResize(ctx: *anyopaque, buf: []u8, buf_align: std.mem.Alignment, new_len: usize, ret_addr: usize) bool {
    _ = ctx;
    _ = buf_align;
    _ = ret_addr;
    if (new_len <= buf.len) return true;
    if (realloc(buf.ptr, new_len)) |new_ptr| {
        return @intFromPtr(new_ptr) == @intFromPtr(buf.ptr);
    }
    return false;
}

fn emFree(ctx: *anyopaque, buf: []u8, buf_align: std.mem.Alignment, ret_addr: usize) void {
    _ = ctx;
    _ = buf_align;
    _ = ret_addr;
    free(buf.ptr);
}

const base_allocator = std.mem.Allocator{
    .ptr = undefined,
    .vtable = &.{
        .alloc = emAlloc,
        .resize = emResize,
        .free = emFree,
        .remap = std.mem.Allocator.noRemap,
    },
};

export fn shadow_parse_source(source_ptr: [*]const u8, source_len: usize, name_ptr: [*]const u8, name_len: usize) ?[*:0]const u8 {
    const source = source_ptr[0..source_len];
    const name = if (name_len > 0) name_ptr[0..name_len] else null;

    const result = Shadow.parseSourceAst(base_allocator, source, name) catch |err| {
        const err_str = @errorName(err);
        const err_with_prefix = std.fmt.allocPrint(base_allocator, "ERROR:{s}", .{err_str}) catch return null;
        const err_msg = base_allocator.dupeZ(u8, err_with_prefix) catch return null;
        base_allocator.free(err_with_prefix);
        return err_msg.ptr;
    };

    const result_z = base_allocator.dupeZ(u8, result) catch return null;
    base_allocator.free(result);
    return result_z.ptr;
}

export fn shadow_init(source_ptr: [*]const u8, source_len: usize) ?*Shadow {
    const source = source_ptr[0..source_len];
    const shadow = base_allocator.create(Shadow) catch return null;
    shadow.* = Shadow.init(base_allocator, source) catch {
        base_allocator.destroy(shadow);
        return null;
    };
    return shadow;
}

export fn shadow_deinit(shadow: *Shadow) void {
    shadow.deinit();
    base_allocator.destroy(shadow);
}

export fn shadow_stitch_into_source(
    shadow: *Shadow,
    target_ptr: [*]const u8,
    target_len: usize,
    source_name_ptr: [*]const u8,
    source_name_len: usize,
    contract_name_ptr: [*]const u8,
    contract_name_len: usize,
) ?[*:0]const u8 {
    const target = target_ptr[0..target_len];
    const source_name = if (source_name_len > 0) source_name_ptr[0..source_name_len] else null;
    const contract_name = if (contract_name_len > 0) contract_name_ptr[0..contract_name_len] else null;

    const result = shadow.stitchIntoSource(target, source_name, contract_name) catch |err| {
        const err_str = @errorName(err);
        const err_with_prefix = std.fmt.allocPrint(base_allocator, "ERROR:{s}", .{err_str}) catch return null;
        defer base_allocator.free(err_with_prefix);
        const err_msg = base_allocator.dupeZ(u8, err_with_prefix) catch return null;
        return err_msg.ptr;
    };

    const result_z = base_allocator.dupeZ(u8, result) catch return null;
    base_allocator.free(result);
    return result_z.ptr;
}

export fn shadow_stitch_into_ast(
    shadow: *Shadow,
    target_ast_ptr: [*]const u8,
    target_ast_len: usize,
    contract_name_ptr: [*]const u8,
    contract_name_len: usize,
) ?[*:0]const u8 {
    const target_ast = target_ast_ptr[0..target_ast_len];
    const contract_name = if (contract_name_len > 0) contract_name_ptr[0..contract_name_len] else null;

    const result = shadow.stitchIntoAst(target_ast, contract_name) catch |err| {
        const err_str = @errorName(err);
        const err_with_prefix = std.fmt.allocPrint(base_allocator, "ERROR:{s}", .{err_str}) catch return null;
        defer base_allocator.free(err_with_prefix);
        const err_msg = base_allocator.dupeZ(u8, err_with_prefix) catch return null;
        return err_msg.ptr;
    };

    const result_z = base_allocator.dupeZ(u8, result) catch return null;
    base_allocator.free(result);
    return result_z.ptr;
}

export fn shadow_free_string(ptr: [*:0]const u8) void {
    const slice = std.mem.span(ptr);
    base_allocator.free(slice);
}
