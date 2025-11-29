const std = @import("std");
const print = std.debug.print;
const Allocator = std.mem.Allocator;
const HashMap = std.HashMap;
const time = std.time;

const FlowRecord = @import("ifr.zig").FlowRecord;

// Cache configuration
pub const FlowCacheConfig = struct {
    max_entries: usize = 10_000_000,
    max_memory: usize = 100 * 1024 * 1024, // 100MB
    eviction_strategy: EvictionStrategy = .LRU,
    ttl_seconds: ?u64 = null, // Time-to-live in seconds
    
    pub const EvictionStrategy = enum {
        LRU,        // Least Recently Used
        LFU,        // Least Frequently Used  
        FIFO,       // First In, First Out
        Random,     // Random eviction
    };
};

// LRU doubly-linked list node
const LRUNode = struct {
    key: [32]u8,
    value: FlowRecord,
    prev: ?*LRUNode,
    next: ?*LRUNode,
    access_count: u64,
    insert_time: u64,
    access_time: u64,
    
    const Self = @This();
    
    pub fn init(allocator: Allocator, key: [32]u8, value: FlowRecord) !*Self {
        const self = try allocator.create(Self);
        const now = @as(u64, @intCast(time.nanoTimestamp()));
        
        self.* = Self{
            .key = key,
            .value = value,
            .prev = null,
            .next = null,
            .access_count = 1,
            .insert_time = now,
            .access_time = now,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self, allocator: Allocator) void {
        allocator.destroy(self);
    }
    
    pub fn updateAccess(self: *Self) void {
        self.access_count += 1;
        self.access_time = @as(u64, @intCast(time.nanoTimestamp()));
    }
    
    pub fn isExpired(self: *Self, ttl_seconds: u64) bool {
        const now = @as(u64, @intCast(time.nanoTimestamp()));
        const ttl_nanos = ttl_seconds * 1_000_000_000;
        return (now - self.insert_time) > ttl_nanos;
    }
};

// High-performance flow cache with multiple eviction strategies
pub const FlowCache = struct {
    allocator: Allocator,
    config: FlowCacheConfig,
    
    // Hash map for O(1) lookup
    entries: HashMap([32]u8, *LRUNode, HashContext, std.hash_map.default_max_load_percentage),
    
    // LRU doubly-linked list
    head: ?*LRUNode,
    tail: ?*LRUNode,
    
    // Statistics
    size: usize,
    memory_used: usize,
    hit_count: u64,
    miss_count: u64,
    eviction_count: u64,
    
    const Self = @This();
    
    const HashContext = struct {
        pub fn hash(self: @This(), key: [32]u8) u64 {
            _ = self;
            return std.hash_map.hashString(&key);
        }
        
        pub fn eql(self: @This(), a: [32]u8, b: [32]u8) bool {
            _ = self;
            return std.mem.eql(u8, &a, &b);
        }
    };
    
    pub fn init(allocator: Allocator, config: FlowCacheConfig) !*Self {
        const self = try allocator.create(Self);
        errdefer allocator.destroy(self);
        
        const entries = HashMap([32]u8, *LRUNode, HashContext, std.hash_map.default_max_load_percentage).init(allocator);
        
        self.* = Self{
            .allocator = allocator,
            .config = config,
            .entries = entries,
            .head = null,
            .tail = null,
            .size = 0,
            .memory_used = 0,
            .hit_count = 0,
            .miss_count = 0,
            .eviction_count = 0,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self) void {
        self.clear();
        self.entries.deinit();
        self.allocator.destroy(self);
    }
    
    // Get flow record from cache
    pub fn get(self: *Self, key: []const u8) ?FlowRecord {
        var key_array: [32]u8 = undefined;
        if (!self.normalizeKey(key, &key_array)) {
            self.miss_count += 1;
            return null;
        }
        
        if (self.entries.get(key_array)) |node| {
            // Check TTL if configured
            if (self.config.ttl_seconds) |ttl| {
                if (node.isExpired(ttl)) {
                    self.removeNode(node);
                    self.miss_count += 1;
                    return null;
                }
            }
            
            // Update access pattern and move to front
            node.updateAccess();
            self.moveToFront(node);
            
            self.hit_count += 1;
            return node.value;
        }
        
        self.miss_count += 1;
        return null;
    }
    
    // Put flow record into cache
    pub fn put(self: *Self, key: []const u8, value: FlowRecord) !void {
        var key_array: [32]u8 = undefined;
        if (!self.normalizeKey(key, &key_array)) {
            return error.InvalidKey;
        }
        
        // Check if key already exists
        if (self.entries.get(key_array)) |existing_node| {
            existing_node.value = value;
            existing_node.updateAccess();
            self.moveToFront(existing_node);
            return;
        }
        
        // Create new node
        const new_node = try LRUNode.init(self.allocator, key_array, value);
        errdefer new_node.deinit(self.allocator);
        
        // Check capacity constraints before adding
        try self.ensureCapacity();
        
        // Add to hash map and LRU list
        try self.entries.put(key_array, new_node);
        self.addToFront(new_node);
        
        self.size += 1;
        self.memory_used += @sizeOf(LRUNode) + @sizeOf(FlowRecord);
    }
    
    // Remove specific key from cache
    pub fn remove(self: *Self, key: []const u8) bool {
        var key_array: [32]u8 = undefined;
        if (!self.normalizeKey(key, &key_array)) {
            return false;
        }
        
        if (self.entries.fetchRemove(key_array)) |entry| {
            self.removeFromList(entry.value);
            entry.value.deinit(self.allocator);
            self.size -= 1;
            self.memory_used -= @sizeOf(LRUNode) + @sizeOf(FlowRecord);
            return true;
        }
        
        return false;
    }
    
    // Clear all entries
    pub fn clear(self: *Self) void {
        while (self.head) |node| {
            self.removeNode(node);
        }
        
        self.size = 0;
        self.memory_used = 0;
    }
    
    // Get cache statistics
    pub fn getStats(self: *Self) Stats {
        const hit_rate = if (self.hit_count + self.miss_count > 0)
            @as(f64, @floatFromInt(self.hit_count)) / @as(f64, @floatFromInt(self.hit_count + self.miss_count))
        else
            0.0;
        
        return Stats{
            .size = self.size,
            .memory_used = self.memory_used,
            .max_entries = self.config.max_entries,
            .max_memory = self.config.max_memory,
            .hit_count = self.hit_count,
            .miss_count = self.miss_count,
            .hit_rate = hit_rate,
            .eviction_count = self.eviction_count,
        };
    }
    
    pub const Stats = struct {
        size: usize,
        memory_used: usize,
        max_entries: usize,
        max_memory: usize,
        hit_count: u64,
        miss_count: u64,
        hit_rate: f64,
        eviction_count: u64,
    };
    
    // Health check
    pub fn isHealthy(self: *Self) bool {
        const stats = self.getStats();
        return stats.memory_used < stats.max_memory and
               stats.size < stats.max_entries and
               stats.hit_rate > 0.5; // At least 50% hit rate for healthy operation
    }
    
    // Cleanup expired entries
    pub fn cleanupExpired(self: *Self) usize {
        if (self.config.ttl_seconds == null) return 0;
        
        const ttl = self.config.ttl_seconds.?;
        var removed_count: usize = 0;
        
        var current = self.tail;
        while (current) |node| {
            const next = node.prev; // Save next before potential removal
            
            if (node.isExpired(ttl)) {
                self.removeNode(node);
                removed_count += 1;
            }
            
            current = next;
        }
        
        return removed_count;
    }
    
    // Private helper methods
    
    fn normalizeKey(self: *Self, key: []const u8, key_array: *[32]u8) bool {
        _ = self;
        
        if (key.len == 0 or key.len > 256) return false;
        
        if (key.len <= 32) {
            @memcpy(key_array[0..key.len], key);
            if (key.len < 32) {
                @memset(key_array[key.len..], 0);
            }
        } else {
            // Hash long keys
            var hasher = std.crypto.hash.blake2.Blake2b256.init(.{});
            hasher.update(key);
            key_array.* = hasher.final();
        }
        
        return true;
    }
    
    fn ensureCapacity(self: *Self) !void {
        // Check memory constraint
        while (self.memory_used >= self.config.max_memory and self.size > 0) {
            try self.evictOne();
        }
        
        // Check entry count constraint
        while (self.size >= self.config.max_entries and self.size > 0) {
            try self.evictOne();
        }
    }
    
    fn evictOne(self: *Self) !void {
        const node_to_evict = switch (self.config.eviction_strategy) {
            .LRU => self.tail,
            .FIFO => self.tail,
            .LFU => self.findLFUNode(),
            .Random => self.findRandomNode(),
        };
        
        if (node_to_evict) |node| {
            self.removeNode(node);
            self.eviction_count += 1;
        }
    }
    
    fn findLFUNode(self: *Self) ?*LRUNode {
        var min_access_count: u64 = std.math.maxInt(u64);
        var lfu_node: ?*LRUNode = null;
        
        var current = self.head;
        while (current) |node| {
            if (node.access_count < min_access_count) {
                min_access_count = node.access_count;
                lfu_node = node;
            }
            current = node.next;
        }
        
        return lfu_node;
    }
    
    fn findRandomNode(self: *Self) ?*LRUNode {
        if (self.size == 0) return null;
        
        var rng = std.rand.DefaultPrng.init(@intCast(time.timestamp()));
        const target_index = rng.random().uintLessThan(usize, self.size);
        
        var current = self.head;
        var index: usize = 0;
        while (current) |node| {
            if (index == target_index) {
                return node;
            }
            current = node.next;
            index += 1;
        }
        
        return self.tail; // Fallback
    }
    
    fn addToFront(self: *Self, node: *LRUNode) void {
        node.next = self.head;
        node.prev = null;
        
        if (self.head) |head| {
            head.prev = node;
        } else {
            self.tail = node;
        }
        
        self.head = node;
    }
    
    fn moveToFront(self: *Self, node: *LRUNode) void {
        if (self.head == node) return; // Already at front
        
        // Remove from current position
        self.removeFromList(node);
        
        // Add to front
        self.addToFront(node);
    }
    
    fn removeFromList(self: *Self, node: *LRUNode) void {
        if (node.prev) |prev| {
            prev.next = node.next;
        } else {
            self.head = node.next;
        }
        
        if (node.next) |next| {
            next.prev = node.prev;
        } else {
            self.tail = node.prev;
        }
        
        node.prev = null;
        node.next = null;
    }
    
    fn removeNode(self: *Self, node: *LRUNode) void {
        // Remove from hash map
        _ = self.entries.remove(node.key);
        
        // Remove from linked list
        self.removeFromList(node);
        
        // Update counters
        self.size -= 1;
        self.memory_used -= @sizeOf(LRUNode) + @sizeOf(FlowRecord);
        
        // Free memory
        node.deinit(self.allocator);
    }
};

// Performance benchmarking
pub fn benchmarkFlowCache(allocator: Allocator, num_operations: usize) !void {
    print("[FlowCache] Running performance benchmark with {} operations\n", .{num_operations});
    
    const cache = try FlowCache.init(allocator, .{
        .max_entries = num_operations,
        .max_memory = 100 * 1024 * 1024, // 100MB
        .eviction_strategy = .LRU,
    });
    defer cache.deinit();
    
    // Prepare test data
    var test_keys = std.ArrayList([]u8).init(allocator);
    defer {
        for (test_keys.items) |key| {
            allocator.free(key);
        }
        test_keys.deinit();
    }
    
    var rng = std.rand.DefaultPrng.init(@intCast(time.timestamp()));
    for (0..num_operations) |i| {
        const key = try std.fmt.allocPrint(allocator, "cache_key_{}_rand_{}", .{ i, rng.random().int(u64) });
        try test_keys.append(key);
    }
    
    // Benchmark insertions
    const insert_start = time.nanoTimestamp();
    for (test_keys.items, 0..) |key, i| {
        const flow = FlowRecord{
            .key = std.mem.zeroes([32]u8),
            .component_id = @as(u32, @intCast(i % 7)),
            .flow_type = .ComponentCommand,
            .timestamp = @intCast(time.nanoTimestamp()),
            .size = 1024,
            .priority = 5,
        };
        try cache.put(key, flow);
    }
    const insert_end = time.nanoTimestamp();
    const insert_duration = insert_end - insert_start;
    
    // Benchmark lookups (cache hits)
    const lookup_start = time.nanoTimestamp();
    var hit_count: usize = 0;
    for (test_keys.items) |key| {
        if (cache.get(key)) |_| {
            hit_count += 1;
        }
    }
    const lookup_end = time.nanoTimestamp();
    const lookup_duration = lookup_end - lookup_start;
    
    // Benchmark misses
    const miss_start = time.nanoTimestamp();
    var miss_count: usize = 0;
    for (0..1000) |i| {
        const nonexistent_key = try std.fmt.allocPrint(allocator, "nonexistent_{}", .{i});
        defer allocator.free(nonexistent_key);
        
        if (cache.get(nonexistent_key)) |_| {
            // Should not happen
        } else {
            miss_count += 1;
        }
    }
    const miss_end = time.nanoTimestamp();
    const miss_duration = miss_end - miss_start;
    
    const insert_ns_per_op = @as(f64, @floatFromInt(insert_duration)) / @as(f64, @floatFromInt(num_operations));
    const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(num_operations));
    const miss_ns_per_op = @as(f64, @floatFromInt(miss_duration)) / 1000.0;
    
    const insert_ops_per_sec = 1_000_000_000.0 / insert_ns_per_op;
    const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
    const miss_ops_per_sec = 1_000_000_000.0 / miss_ns_per_op;
    
    const stats = cache.getStats();
    
    print("[FlowCache] Benchmark Results:\n");
    print("  Insert: {:.2} ns/op, {:.0} ops/sec\n", .{ insert_ns_per_op, insert_ops_per_sec });
    print("  Lookup (hits): {:.2} ns/op, {:.0} ops/sec\n", .{ lookup_ns_per_op, lookup_ops_per_sec });
    print("  Lookup (misses): {:.2} ns/op, {:.0} ops/sec\n", .{ miss_ns_per_op, miss_ops_per_sec });
    print("  Cache hits: {}/{} ({:.1}%)\n", .{ hit_count, num_operations, @as(f64, @floatFromInt(hit_count * 100)) / @as(f64, @floatFromInt(num_operations)) });
    print("  Cache misses: {} (expected)\n", .{miss_count});
    print("  Hit rate: {:.3}%\n", .{ stats.hit_rate * 100.0 });
    print("  Memory usage: {:.2} MB / {:.2} MB\n", .{ 
        @as(f64, @floatFromInt(stats.memory_used)) / (1024.0 * 1024.0),
        @as(f64, @floatFromInt(stats.max_memory)) / (1024.0 * 1024.0)
    });
    print("  Evictions: {}\n", .{stats.eviction_count});
    
    // Verify performance targets
    if (lookup_ns_per_op > 100.0) {
        print("[FlowCache] WARNING: Cache lookup latency {:.2} ns exceeds 100ns target!\n", .{lookup_ns_per_op});
    } else {
        print("[FlowCache] ✓ Cache lookup latency {:.2} ns meets target\n", .{lookup_ns_per_op});
    }
    
    if (stats.hit_rate < 0.95) {
        print("[FlowCache] WARNING: Hit rate {:.1}% below 95% target!\n", .{ stats.hit_rate * 100.0 });
    } else {
        print("[FlowCache] ✓ Hit rate {:.1}% meets target\n", .{ stats.hit_rate * 100.0 });
    }
}

// Unit tests
test "FlowCache basic operations" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const cache = try FlowCache.init(allocator, .{});
    defer cache.deinit();
    
    // Test put and get
    const test_key = "test_key";
    const test_flow = FlowRecord{
        .key = std.mem.zeroes([32]u8),
        .component_id = 1,
        .flow_type = .ComponentCommand,
        .timestamp = @intCast(time.nanoTimestamp()),
        .size = 1024,
        .priority = 5,
    };
    
    try cache.put(test_key, test_flow);
    
    const result = cache.get(test_key);
    try testing.expect(result != null);
    try testing.expect(result.?.component_id == 1);
    
    // Test miss
    const no_result = cache.get("nonexistent");
    try testing.expect(no_result == null);
    
    // Test removal
    try testing.expect(cache.remove(test_key));
    try testing.expect(!cache.remove(test_key));
    
    const removed_result = cache.get(test_key);
    try testing.expect(removed_result == null);
}

test "FlowCache LRU eviction" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const cache = try FlowCache.init(allocator, .{
        .max_entries = 3,
        .eviction_strategy = .LRU,
    });
    defer cache.deinit();
    
    // Fill cache to capacity
    for (0..3) |i| {
        const key = try std.fmt.allocPrint(allocator, "key_{}", .{i});
        defer allocator.free(key);
        
        const flow = FlowRecord{
            .key = std.mem.zeroes([32]u8),
            .component_id = @as(u32, @intCast(i)),
            .flow_type = .ComponentCommand,
            .timestamp = @intCast(time.nanoTimestamp()),
            .size = 1024,
            .priority = 5,
        };
        
        try cache.put(key, flow);
    }
    
    const stats1 = cache.getStats();
    try testing.expect(stats1.size == 3);
    
    // Add one more item - should evict LRU
    const new_key = "key_new";
    const new_flow = FlowRecord{
        .key = std.mem.zeroes([32]u8),
        .component_id = 999,
        .flow_type = .ComponentCommand,
        .timestamp = @intCast(time.nanoTimestamp()),
        .size = 1024,
        .priority = 5,
    };
    
    try cache.put(new_key, new_flow);
    
    const stats2 = cache.getStats();
    try testing.expect(stats2.size == 3);
    try testing.expect(stats2.eviction_count == 1);
    
    // New item should be present
    const new_result = cache.get(new_key);
    try testing.expect(new_result != null);
    try testing.expect(new_result.?.component_id == 999);
}

test "FlowCache TTL expiration" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const cache = try FlowCache.init(allocator, .{
        .ttl_seconds = 1, // 1 second TTL
    });
    defer cache.deinit();
    
    const test_key = "ttl_key";
    const test_flow = FlowRecord{
        .key = std.mem.zeroes([32]u8),
        .component_id = 1,
        .flow_type = .ComponentCommand,
        .timestamp = @intCast(time.nanoTimestamp()),
        .size = 1024,
        .priority = 5,
    };
    
    try cache.put(test_key, test_flow);
    
    // Should be present immediately
    const result1 = cache.get(test_key);
    try testing.expect(result1 != null);
    
    // Wait for expiration
    std.time.sleep(1_100_000_000); // 1.1 seconds
    
    // Should be expired
    const result2 = cache.get(test_key);
    try testing.expect(result2 == null);
}

test "FlowCache performance" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    try benchmarkFlowCache(allocator, 10000);
}