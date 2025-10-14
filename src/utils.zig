const std = @import("std");

pub const Utils = struct {
    /// Find maximum ID in AST JSON tree
    pub fn findMaxId(value: std.json.Value) i64 {
        var max_id: i64 = 0;

        switch (value) {
            .object => |obj| {
                if (obj.get("id")) |id_value| {
                    if (id_value == .integer) {
                        max_id = @max(max_id, id_value.integer);
                    }
                }
                var it = obj.iterator();
                while (it.next()) |entry| {
                    max_id = @max(max_id, findMaxId(entry.value_ptr.*));
                }
            },
            .array => |arr| {
                for (arr.items) |item| {
                    max_id = @max(max_id, findMaxId(item));
                }
            },
            else => {},
        }

        return max_id;
    }

    /// Renumber all IDs in AST JSON tree by adding offset to each ID
    pub fn renumberIds(value: *std.json.Value, offset: i64) void {
        switch (value.*) {
            .object => |*obj| {
                if (obj.getPtr("id")) |id_value| {
                    if (id_value.* == .integer) {
                        id_value.integer += offset;
                    }
                }
                var it = obj.iterator();
                while (it.next()) |entry| {
                    renumberIds(entry.value_ptr, offset);
                }
            },
            .array => |*arr| {
                for (arr.items) |*item| {
                    renumberIds(item, offset);
                }
            },
            else => {},
        }
    }
};
