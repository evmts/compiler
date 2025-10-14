const std = @import("std");
const Shadow = @import("src/shadow.zig").Shadow;

const allocator = std.heap.wasm_allocator;

export fn shadow_parse_source(source_ptr: [*]const u8, source_len: usize, name_ptr: [*]const u8, name_len: usize) ?[*:0]const u8 {
    const source = source_ptr[0..source_len];
    const name = if (name_len > 0) name_ptr[0..name_len] else null;

    const result = Shadow.parseSourceAst(allocator, source, name) catch |err| {
        const err_str = @errorName(err);
        const err_msg = allocator.dupeZ(u8, err_str) catch return null;
        return err_msg.ptr;
    };

    const result_z = allocator.dupeZ(u8, result) catch return null;
    allocator.free(result);
    return result_z.ptr;
}

export fn shadow_init(source_ptr: [*]const u8, source_len: usize) ?*Shadow {
    const source = source_ptr[0..source_len];
    const shadow = allocator.create(Shadow) catch return null;
    shadow.* = Shadow.init(allocator, source) catch {
        allocator.destroy(shadow);
        return null;
    };
    return shadow;
}

export fn shadow_deinit(shadow: *Shadow) void {
    shadow.deinit();
    allocator.destroy(shadow);
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
        const err_msg = allocator.dupeZ(u8, err_str) catch return null;
        return err_msg.ptr;
    };

    const result_z = allocator.dupeZ(u8, result) catch return null;
    allocator.free(result);
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
        const err_msg = allocator.dupeZ(u8, err_str) catch return null;
        return err_msg.ptr;
    };

    const result_z = allocator.dupeZ(u8, result) catch return null;
    allocator.free(result);
    return result_z.ptr;
}

export fn shadow_free_string(ptr: [*:0]const u8) void {
    const slice = std.mem.span(ptr);
    allocator.free(slice);
}
