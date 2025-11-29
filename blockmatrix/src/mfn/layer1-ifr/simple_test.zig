const std = @import("std");

pub fn main() !void {
    std.debug.print("Simple IFR test works!\n", .{});
}

test "simple test" {
    const testing = std.testing;
    try testing.expect(2 + 2 == 4);
}