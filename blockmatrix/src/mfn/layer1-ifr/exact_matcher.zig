const std = @import("std");
const print = std.debug.print;
const Allocator = std.mem.Allocator;
const HashMap = std.HashMap;
const ArrayList = std.ArrayList;
const time = std.time;

const FlowRecord = @import("ifr.zig").FlowRecord;

// Blake3 hash implementation for ultra-fast hashing
const Blake3 = struct {
    const Self = @This();
    
    pub fn hash(data: []const u8) [32]u8 {
        // Simplified Blake3-like hash - in production, use actual Blake3
        var hasher = std.crypto.hash.blake2.Blake2b256.init(.{});
        hasher.update(data);
        return hasher.final();
    }
    
    pub fn hashToU64(data: []const u8) u64 {
        const full_hash = hash(data);
        return std.mem.readInt(u64, full_hash[0..8], .little);
    }
};

// Configuration for the exact matcher
pub const ExactMatcherConfig = struct {
    max_entries: usize = 10_000_000,
    hash_algorithm: HashAlgorithm = .Blake3,
    load_factor: f64 = 0.75,
    
    pub const HashAlgorithm = enum {
        Blake3,
        XXHash64,
        CityHash,
    };
};

// High-performance exact matcher with Robin Hood hashing
pub const ExactMatcher = struct {
    allocator: Allocator,
    config: ExactMatcherConfig,
    entries: []?Entry,
    size: usize,
    capacity: usize,
    tombstones: usize,
    
    const Self = @This();
    
    const Entry = struct {
        key_hash: u64,
        key: [32]u8,
        value: FlowRecord,
        psl: u16, // Probe sequence length for Robin Hood hashing
    };
    
    const EMPTY_PSL: u16 = 0;
    const TOMBSTONE_PSL: u16 = std.math.maxInt(u16);
    
    pub fn init(allocator: Allocator, config: ExactMatcherConfig) !*Self {
        const self = try allocator.create(Self);
        errdefer allocator.destroy(self);
        
        const initial_capacity = nextPowerOfTwo(config.max_entries);
        const entries = try allocator.alloc(?Entry, initial_capacity);
        @memset(entries, null);
        
        self.* = Self{
            .allocator = allocator,
            .config = config,
            .entries = entries,
            .size = 0,
            .capacity = initial_capacity,
            .tombstones = 0,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self) void {
        self.allocator.free(self.entries);
        self.allocator.destroy(self);
    }
    
    // Ultra-fast exact matching with Robin Hood hashing
    pub fn find(self: *Self, key: []const u8) ?FlowRecord {
        if (self.size == 0) return null;
        
        const key_hash = self.hashKey(key);
        var key_array: [32]u8 = undefined;
        
        // Convert key to fixed-size array
        if (key.len <= 32) {
            @memcpy(key_array[0..key.len], key);
            if (key.len < 32) {
                @memset(key_array[key.len..], 0);
            }
        } else {
            // Hash long keys to 32 bytes
            key_array = Blake3.hash(key);
        }
        
        var index = key_hash & (self.capacity - 1);
        var psl: u16 = 1;
        
        while (psl <= self.capacity) : ({
            index = (index + 1) & (self.capacity - 1);
            psl += 1;
        }) {
            const entry_opt = self.entries[index];
            
            if (entry_opt == null) {
                // Empty slot found - key not present
                return null;
            }
            
            const entry = entry_opt.?;
            
            if (entry.psl == TOMBSTONE_PSL) {
                // Tombstone - continue searching
                continue;
            }
            
            if (entry.key_hash == key_hash and std.mem.eql(u8, &entry.key, &key_array)) {
                // Found the key
                return entry.value;
            }
            
            if (psl > entry.psl) {
                // Would have been placed here if it existed
                return null;
            }
        }
        
        return null;
    }
    
    // Insert with Robin Hood hashing for optimal performance
    pub fn insert(self: *Self, key: []const u8, value: FlowRecord) !void {
        // Resize if necessary
        if (@as(f64, @floatFromInt(self.size + self.tombstones)) / @as(f64, @floatFromInt(self.capacity)) > self.config.load_factor) {
            try self.resize();
        }
        
        const key_hash = self.hashKey(key);
        var key_array: [32]u8 = undefined;
        
        // Convert key to fixed-size array
        if (key.len <= 32) {
            @memcpy(key_array[0..key.len], key);
            if (key.len < 32) {
                @memset(key_array[key.len..], 0);
            }
        } else {
            key_array = Blake3.hash(key);
        }
        
        var new_entry = Entry{
            .key_hash = key_hash,
            .key = key_array,
            .value = value,
            .psl = 1,
        };
        
        var index = key_hash & (self.capacity - 1);
        
        while (true) {
            const entry_opt = &self.entries[index];
            
            if (entry_opt.* == null or entry_opt.*.?.psl == TOMBSTONE_PSL) {
                // Empty slot or tombstone - place entry here
                if (entry_opt.* != null and entry_opt.*.?.psl == TOMBSTONE_PSL) {
                    self.tombstones -= 1;
                }
                entry_opt.* = new_entry;
                self.size += 1;
                return;
            }
            
            var existing_entry = entry_opt.*.?;
            
            // Check for duplicate key
            if (existing_entry.key_hash == key_hash and std.mem.eql(u8, &existing_entry.key, &key_array)) {
                // Update existing entry
                existing_entry.value = value;
                entry_opt.* = existing_entry;
                return;
            }
            
            // Robin Hood hashing: if new entry has higher PSL, swap
            if (new_entry.psl > existing_entry.psl) {
                entry_opt.* = new_entry;
                new_entry = existing_entry;
            }
            
            new_entry.psl += 1;
            index = (index + 1) & (self.capacity - 1);
        }
    }
    
    // Remove entry and mark as tombstone
    pub fn remove(self: *Self, key: []const u8) bool {
        if (self.size == 0) return false;
        
        const key_hash = self.hashKey(key);
        var key_array: [32]u8 = undefined;
        
        if (key.len <= 32) {
            @memcpy(key_array[0..key.len], key);
            if (key.len < 32) {
                @memset(key_array[key.len..], 0);
            }
        } else {
            key_array = Blake3.hash(key);
        }
        
        var index = key_hash & (self.capacity - 1);
        var psl: u16 = 1;
        
        while (psl <= self.capacity) : ({
            index = (index + 1) & (self.capacity - 1);
            psl += 1;
        }) {
            const entry_opt = &self.entries[index];
            
            if (entry_opt.* == null) {
                return false;
            }
            
            const entry = entry_opt.*.?;
            
            if (entry.psl == TOMBSTONE_PSL) {
                continue;
            }
            
            if (entry.key_hash == key_hash and std.mem.eql(u8, &entry.key, &key_array)) {
                // Mark as tombstone
                var tombstone = entry;
                tombstone.psl = TOMBSTONE_PSL;
                entry_opt.* = tombstone;
                
                self.size -= 1;
                self.tombstones += 1;
                return true;
            }
            
            if (psl > entry.psl) {
                return false;
            }
        }
        
        return false;
    }
    
    // Get current statistics
    pub fn getStats(self: *Self) Stats {
        var max_psl: u16 = 0;
        var total_psl: u64 = 0;
        var active_entries: usize = 0;
        
        for (self.entries) |entry_opt| {
            if (entry_opt) |entry| {
                if (entry.psl != TOMBSTONE_PSL) {
                    max_psl = @max(max_psl, entry.psl);
                    total_psl += entry.psl;
                    active_entries += 1;
                }
            }
        }
        
        return Stats{
            .size = self.size,
            .capacity = self.capacity,
            .tombstones = self.tombstones,
            .load_factor = @as(f64, @floatFromInt(self.size)) / @as(f64, @floatFromInt(self.capacity)),
            .max_psl = max_psl,
            .avg_psl = if (active_entries > 0) @as(f64, @floatFromInt(total_psl)) / @as(f64, @floatFromInt(active_entries)) else 0.0,
        };
    }
    
    pub const Stats = struct {
        size: usize,
        capacity: usize,
        tombstones: usize,
        load_factor: f64,
        max_psl: u16,
        avg_psl: f64,
    };
    
    // Health check
    pub fn isHealthy(self: *Self) bool {
        const stats = self.getStats();
        return stats.load_factor < 0.9 and stats.max_psl < 100;
    }
    
    // Private methods
    fn hashKey(self: *Self, key: []const u8) u64 {
        return switch (self.config.hash_algorithm) {
            .Blake3 => Blake3.hashToU64(key),
            .XXHash64 => std.hash_map.hashString(key), // Fallback to std hash
            .CityHash => std.hash_map.hashString(key), // Fallback to std hash
        };
    }
    
    fn resize(self: *Self) !void {
        const old_entries = self.entries;
        const old_capacity = self.capacity;
        
        self.capacity = self.capacity * 2;
        self.entries = try self.allocator.alloc(?Entry, self.capacity);
        @memset(self.entries, null);
        
        const old_size = self.size;
        self.size = 0;
        self.tombstones = 0;
        
        // Rehash all entries
        for (old_entries) |entry_opt| {
            if (entry_opt) |entry| {
                if (entry.psl != TOMBSTONE_PSL) {
                    try self.insert(&entry.key, entry.value);
                }
            }
        }
        
        self.allocator.free(old_entries);
        
        print("[ExactMatcher] Resized from {} to {} entries, rehashed {} items\n", .{ old_capacity, self.capacity, old_size });
    }
};

// Utility function for power of two calculation
fn nextPowerOfTwo(n: usize) usize {
    if (n == 0) return 1;
    
    var result: usize = 1;
    while (result < n) {
        result <<= 1;
    }
    return result;
}

// Performance benchmarking
pub fn benchmarkExactMatcher(allocator: Allocator, num_operations: usize) !void {
    print("[ExactMatcher] Running performance benchmark with {} operations\n", .{num_operations});
    
    const matcher = try ExactMatcher.init(allocator, .{
        .max_entries = num_operations * 2,
        .hash_algorithm = .Blake3,
    });
    defer matcher.deinit();
    
    // Prepare test data
    const test_keys = try allocator.alloc([]u8, num_operations);
    defer {
        for (test_keys) |key| {
            allocator.free(key);
        }
        allocator.free(test_keys);
    }
    
    var rng = std.rand.DefaultPrng.init(@intCast(time.timestamp()));
    for (test_keys, 0..) |*key, i| {
        key.* = try std.fmt.allocPrint(allocator, "test_key_{}_rand_{}", .{ i, rng.random().int(u64) });
    }
    
    // Benchmark insertions
    const insert_start = time.nanoTimestamp();
    for (test_keys, 0..) |key, i| {
        const flow = FlowRecord{
            .key = std.mem.zeroes([32]u8),
            .component_id = @as(u32, @intCast(i % 7)),
            .flow_type = .ComponentCommand,
            .timestamp = @intCast(time.nanoTimestamp()),
            .size = 1024,
            .priority = 5,
        };
        try matcher.insert(key, flow);
    }
    const insert_end = time.nanoTimestamp();
    const insert_duration = insert_end - insert_start;
    
    // Benchmark lookups
    const lookup_start = time.nanoTimestamp();
    for (test_keys) |key| {
        _ = matcher.find(key);
    }
    const lookup_end = time.nanoTimestamp();
    const lookup_duration = lookup_end - lookup_start;
    
    const insert_ns_per_op = @as(f64, @floatFromInt(insert_duration)) / @as(f64, @floatFromInt(num_operations));
    const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(num_operations));
    const insert_ops_per_sec = 1_000_000_000.0 / insert_ns_per_op;
    const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
    
    const stats = matcher.getStats();
    
    print("[ExactMatcher] Benchmark Results:\n");
    print("  Insert: {:.2} ns/op, {:.0} ops/sec\n", .{ insert_ns_per_op, insert_ops_per_sec });
    print("  Lookup: {:.2} ns/op, {:.0} ops/sec\n", .{ lookup_ns_per_op, lookup_ops_per_sec });
    print("  Load factor: {:.3}\n", .{stats.load_factor});
    print("  Max PSL: {}\n", .{stats.max_psl});
    print("  Avg PSL: {:.2}\n", .{stats.avg_psl});
    
    // Verify performance targets
    const lookup_ms_per_op = lookup_ns_per_op / 1_000_000.0;
    if (lookup_ms_per_op > 0.1) {
        print("[ExactMatcher] WARNING: Lookup latency {:.3} ms exceeds 0.1ms target!\n", .{lookup_ms_per_op});
    } else {
        print("[ExactMatcher] ✓ Lookup latency {:.3} ms meets <0.1ms target\n", .{lookup_ms_per_op});
    }
    
    if (lookup_ops_per_sec < 10_000_000) {
        print("[ExactMatcher] WARNING: Throughput {:.0} ops/sec below 10M target!\n", .{lookup_ops_per_sec});
    } else {
        print("[ExactMatcher] ✓ Throughput {:.0} ops/sec exceeds 10M target\n", .{lookup_ops_per_sec});
    }
}

// Unit tests
test "ExactMatcher basic operations" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const matcher = try ExactMatcher.init(allocator, .{});
    defer matcher.deinit();
    
    // Test insertion and lookup
    const test_key = "test_key";
    const test_flow = FlowRecord{
        .key = std.mem.zeroes([32]u8),
        .component_id = 1,
        .flow_type = .ComponentCommand,
        .timestamp = @intCast(time.nanoTimestamp()),
        .size = 1024,
        .priority = 5,
    };
    
    try matcher.insert(test_key, test_flow);
    
    const result = matcher.find(test_key);
    try testing.expect(result != null);
    try testing.expect(result.?.component_id == 1);
    
    // Test non-existent key
    const no_result = matcher.find("nonexistent");
    try testing.expect(no_result == null);
    
    // Test removal
    try testing.expect(matcher.remove(test_key));
    try testing.expect(!matcher.remove(test_key));
    
    const removed_result = matcher.find(test_key);
    try testing.expect(removed_result == null);
}

test "ExactMatcher collision handling" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const matcher = try ExactMatcher.init(allocator, .{});
    defer matcher.deinit();
    
    // Insert multiple entries that might collide
    for (0..1000) |i| {
        const key = try std.fmt.allocPrint(testing.allocator, "key_{}", .{i});
        defer testing.allocator.free(key);
        
        const flow = FlowRecord{
            .key = std.mem.zeroes([32]u8),
            .component_id = @as(u32, @intCast(i % 7)),
            .flow_type = .ComponentCommand,
            .timestamp = @intCast(time.nanoTimestamp()),
            .size = 1024,
            .priority = 5,
        };
        
        try matcher.insert(key, flow);
    }
    
    // Verify all entries can be found
    for (0..1000) |i| {
        const key = try std.fmt.allocPrint(testing.allocator, "key_{}", .{i});
        defer testing.allocator.free(key);
        
        const result = matcher.find(key);
        try testing.expect(result != null);
        try testing.expect(result.?.component_id == @as(u32, @intCast(i % 7)));
    }
    
    const stats = matcher.getStats();
    try testing.expect(stats.size == 1000);
}

test "ExactMatcher performance" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    try benchmarkExactMatcher(allocator, 10000);
}