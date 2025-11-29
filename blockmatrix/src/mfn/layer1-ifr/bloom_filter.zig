const std = @import("std");
const print = std.debug.print;
const Allocator = std.mem.Allocator;
const ArrayList = std.ArrayList;
const time = std.time;
const math = std.math;

// Configuration for bloom filter bank
pub const BloomFilterConfig = struct {
    false_positive_rate: f64 = 0.01,
    expected_entries: usize = 1_000_000,
    hash_functions: u8 = 3,
    max_filters: usize = 16, // For rotating filters
};

// High-performance bloom filter implementation
pub const BloomFilter = struct {
    bit_array: []u8,
    size_bits: usize,
    hash_functions: u8,
    entries_added: usize,
    
    const Self = @This();
    
    pub fn init(allocator: Allocator, size_bits: usize, hash_functions: u8) !*Self {
        const self = try allocator.create(Self);
        errdefer allocator.destroy(self);
        
        const size_bytes = (size_bits + 7) / 8;
        const bit_array = try allocator.alloc(u8, size_bytes);
        @memset(bit_array, 0);
        
        self.* = Self{
            .bit_array = bit_array,
            .size_bits = size_bits,
            .hash_functions = hash_functions,
            .entries_added = 0,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self, allocator: Allocator) void {
        allocator.free(self.bit_array);
        allocator.destroy(self);
    }
    
    // Add element to bloom filter
    pub fn add(self: *Self, key: []const u8) void {
        const hashes = self.generateHashes(key);
        
        for (0..self.hash_functions) |i| {
            const bit_index = hashes[i] % self.size_bits;
            const byte_index = bit_index / 8;
            const bit_offset = @as(u3, @intCast(bit_index % 8));
            
            self.bit_array[byte_index] |= (@as(u8, 1) << bit_offset);
        }
        
        self.entries_added += 1;
    }
    
    // Check if element might be in the set
    pub fn contains(self: *Self, key: []const u8) bool {
        const hashes = self.generateHashes(key);
        
        for (0..self.hash_functions) |i| {
            const bit_index = hashes[i] % self.size_bits;
            const byte_index = bit_index / 8;
            const bit_offset = @as(u3, @intCast(bit_index % 8));
            
            const bit_set = (self.bit_array[byte_index] & (@as(u8, 1) << bit_offset)) != 0;
            if (!bit_set) {
                return false;
            }
        }
        
        return true;
    }
    
    // Clear all bits
    pub fn clear(self: *Self) void {
        @memset(self.bit_array, 0);
        self.entries_added = 0;
    }
    
    // Get false positive probability based on current load
    pub fn getFalsePositiveRate(self: *Self) f64 {
        if (self.entries_added == 0) return 0.0;
        
        const k = @as(f64, @floatFromInt(self.hash_functions));
        const m = @as(f64, @floatFromInt(self.size_bits));
        const n = @as(f64, @floatFromInt(self.entries_added));
        
        // (1 - e^(-k*n/m))^k
        const exponent = -k * n / m;
        const base = 1.0 - @exp(exponent);
        return math.pow(f64, base, k);
    }
    
    // Generate multiple hash values using double hashing
    fn generateHashes(self: *Self, key: []const u8) [16]usize {
        const hash1 = std.hash_map.hashString(key);
        const hash2 = std.hash.XxHash64.hash(0, key);
        
        var hashes: [16]usize = undefined;
        for (0..self.hash_functions) |i| {
            hashes[i] = @intCast((hash1 +% (@as(u64, @intCast(i)) *% hash2)) & 0x7FFFFFFFFFFFFFFF);
        }
        
        return hashes;
    }
};

// Bank of rotating bloom filters for better accuracy
pub const BloomFilterBank = struct {
    allocator: Allocator,
    config: BloomFilterConfig,
    filters: []*BloomFilter,
    current_filter: usize,
    size_bits_per_filter: usize,
    
    const Self = @This();
    
    pub fn init(allocator: Allocator, config: BloomFilterConfig) !*Self {
        const self = try allocator.create(Self);
        errdefer allocator.destroy(self);
        
        // Calculate optimal bit array size using the formula:
        // m = -n * ln(p) / (ln(2)^2)
        const n = @as(f64, @floatFromInt(config.expected_entries));
        const p = config.false_positive_rate;
        const m = -n * @log(p) / (math.ln2 * math.ln2);
        const size_bits_per_filter = @as(usize, @intFromFloat(@max(1024, m)));
        
        print("[BloomFilterBank] Initializing with {} filters, {} bits each\n", .{ config.max_filters, size_bits_per_filter });
        
        const filters = try allocator.alloc(*BloomFilter, config.max_filters);
        errdefer allocator.free(filters);
        
        // Initialize all filters
        for (filters, 0..) |*filter, i| {
            filter.* = BloomFilter.init(allocator, size_bits_per_filter, config.hash_functions) catch |err| {
                // Cleanup already initialized filters
                for (filters[0..i]) |f| {
                    f.deinit(allocator);
                }
                allocator.free(filters);
                return err;
            };
        }
        
        self.* = Self{
            .allocator = allocator,
            .config = config,
            .filters = filters,
            .current_filter = 0,
            .size_bits_per_filter = size_bits_per_filter,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self) void {
        for (self.filters) |filter| {
            filter.deinit(self.allocator);
        }
        self.allocator.free(self.filters);
        self.allocator.destroy(self);
    }
    
    // Add element to the current filter
    pub fn add(self: *Self, key: []const u8) void {
        // Add to current filter
        self.filters[self.current_filter].add(key);
        
        // Check if we should rotate to next filter
        const current_rate = self.filters[self.current_filter].getFalsePositiveRate();
        if (current_rate > self.config.false_positive_rate * 1.5) {
            self.rotateFilter();
        }
    }
    
    // Check if element might be in any filter
    pub fn contains(self: *Self, key: []const u8) bool {
        // Check all filters - element could be in any of them
        for (self.filters) |filter| {
            if (filter.contains(key)) {
                return true;
            }
        }
        return false;
    }
    
    // Get statistics across all filters
    pub fn getStats(self: *Self) Stats {
        var total_entries: usize = 0;
        var max_fpr: f64 = 0.0;
        var avg_fpr: f64 = 0.0;
        var active_filters: usize = 0;
        
        for (self.filters) |filter| {
            if (filter.entries_added > 0) {
                total_entries += filter.entries_added;
                const fpr = filter.getFalsePositiveRate();
                max_fpr = @max(max_fpr, fpr);
                avg_fpr += fpr;
                active_filters += 1;
            }
        }
        
        if (active_filters > 0) {
            avg_fpr /= @as(f64, @floatFromInt(active_filters));
        }
        
        return Stats{
            .total_entries = total_entries,
            .active_filters = active_filters,
            .current_filter = self.current_filter,
            .max_false_positive_rate = max_fpr,
            .avg_false_positive_rate = avg_fpr,
            .memory_usage_bytes = self.getMemoryUsage(),
        };
    }
    
    pub const Stats = struct {
        total_entries: usize,
        active_filters: usize,
        current_filter: usize,
        max_false_positive_rate: f64,
        avg_false_positive_rate: f64,
        memory_usage_bytes: usize,
    };
    
    // Clear all filters
    pub fn clear(self: *Self) void {
        for (self.filters) |filter| {
            filter.clear();
        }
        self.current_filter = 0;
    }
    
    // Get total memory usage
    pub fn getMemoryUsage(self: *Self) usize {
        const bits_per_filter = self.size_bits_per_filter;
        const bytes_per_filter = (bits_per_filter + 7) / 8;
        return self.filters.len * bytes_per_filter;
    }
    
    // Health check
    pub fn isHealthy(self: *Self) bool {
        const stats = self.getStats();
        return stats.max_false_positive_rate < self.config.false_positive_rate * 2.0;
    }
    
    // Rotate to next filter when current one gets too full
    fn rotateFilter(self: *Self) void {
        const old_filter = self.current_filter;
        self.current_filter = (self.current_filter + 1) % self.filters.len;
        
        // Clear the oldest filter if we've wrapped around
        if (self.current_filter == 0 and self.filters[self.filters.len - 1].entries_added > 0) {
            const oldest_filter = (old_filter + 1) % self.filters.len;
            self.filters[oldest_filter].clear();
            print("[BloomFilterBank] Rotated to filter {} (cleared oldest)\n", .{self.current_filter});
        } else {
            print("[BloomFilterBank] Rotated to filter {}\n", .{self.current_filter});
        }
    }
};

// Calculate optimal bloom filter parameters
pub fn calculateOptimalParams(expected_entries: usize, false_positive_rate: f64) struct {
    size_bits: usize,
    hash_functions: u8,
} {
    const n = @as(f64, @floatFromInt(expected_entries));
    const p = false_positive_rate;
    
    // Optimal size: m = -n * ln(p) / (ln(2)^2)
    const m = -n * @log(p) / (math.ln2 * math.ln2);
    const size_bits = @as(usize, @intFromFloat(@max(1024, m)));
    
    // Optimal hash functions: k = m/n * ln(2)
    const k = (m / n) * math.ln2;
    const hash_functions = @as(u8, @intFromFloat(@max(1, @min(16, @round(k)))));
    
    return .{
        .size_bits = size_bits,
        .hash_functions = hash_functions,
    };
}

// Benchmark bloom filter performance
pub fn benchmarkBloomFilter(allocator: Allocator, num_operations: usize) !void {
    print("[BloomFilter] Running performance benchmark with {} operations\n", .{num_operations});
    
    const filter_bank = try BloomFilterBank.init(allocator, .{
        .false_positive_rate = 0.01,
        .expected_entries = num_operations,
        .hash_functions = 3,
        .max_filters = 4,
    });
    defer filter_bank.deinit();
    
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
        key.* = try std.fmt.allocPrint(allocator, "bloom_key_{}_rand_{}", .{ i, rng.random().int(u64) });
    }
    
    // Benchmark additions
    const add_start = time.nanoTimestamp();
    for (test_keys) |key| {
        filter_bank.add(key);
    }
    const add_end = time.nanoTimestamp();
    const add_duration = add_end - add_start;
    
    // Benchmark lookups (existing keys)
    const lookup_start = time.nanoTimestamp();
    var found_count: usize = 0;
    for (test_keys) |key| {
        if (filter_bank.contains(key)) {
            found_count += 1;
        }
    }
    const lookup_end = time.nanoTimestamp();
    const lookup_duration = lookup_end - lookup_start;
    
    // Test false positives with non-existent keys
    var false_positives: usize = 0;
    for (0..num_operations / 10) |i| {
        const nonexistent_key = try std.fmt.allocPrint(allocator, "nonexistent_{}", .{i});
        defer allocator.free(nonexistent_key);
        
        if (filter_bank.contains(nonexistent_key)) {
            false_positives += 1;
        }
    }
    
    const add_ns_per_op = @as(f64, @floatFromInt(add_duration)) / @as(f64, @floatFromInt(num_operations));
    const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(num_operations));
    const add_ops_per_sec = 1_000_000_000.0 / add_ns_per_op;
    const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
    
    const stats = filter_bank.getStats();
    const actual_fpr = @as(f64, @floatFromInt(false_positives)) / @as(f64, @floatFromInt(num_operations / 10));
    
    print("[BloomFilter] Benchmark Results:\n");
    print("  Add: {:.2} ns/op, {:.0} ops/sec\n", .{ add_ns_per_op, add_ops_per_sec });
    print("  Lookup: {:.2} ns/op, {:.0} ops/sec\n", .{ lookup_ns_per_op, lookup_ops_per_sec });
    print("  Found existing: {}/{} ({}%)\n", .{ found_count, num_operations, (found_count * 100) / num_operations });
    print("  False positives: {}/{} ({:.3}%)\n", .{ false_positives, num_operations / 10, actual_fpr * 100.0 });
    print("  Target FPR: {:.3}%, Actual FPR: {:.3}%\n", .{ filter_bank.config.false_positive_rate * 100.0, actual_fpr * 100.0 });
    print("  Memory usage: {:.2} MB\n", .{ @as(f64, @floatFromInt(stats.memory_usage_bytes)) / (1024.0 * 1024.0) });
    print("  Active filters: {}\n", .{stats.active_filters});
    
    // Verify performance targets
    if (actual_fpr > filter_bank.config.false_positive_rate * 2.0) {
        print("[BloomFilter] WARNING: False positive rate {:.3}% exceeds target!\n", .{ actual_fpr * 100.0 });
    } else {
        print("[BloomFilter] ✓ False positive rate within target\n");
    }
}

// Unit tests
test "BloomFilter basic operations" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const filter = try BloomFilter.init(allocator, 1024 * 8, 3);
    defer filter.deinit(allocator);
    
    // Test basic add and contains
    const test_key = "test_key";
    filter.add(test_key);
    try testing.expect(filter.contains(test_key));
    
    // Test that non-added key likely returns false
    try testing.expect(!filter.contains("nonexistent_key"));
    
    // Test clear
    filter.clear();
    try testing.expect(filter.entries_added == 0);
}

test "BloomFilterBank operations" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const filter_bank = try BloomFilterBank.init(allocator, .{
        .false_positive_rate = 0.01,
        .expected_entries = 1000,
        .hash_functions = 3,
        .max_filters = 4,
    });
    defer filter_bank.deinit();
    
    // Add many keys
    for (0..1000) |i| {
        const key = try std.fmt.allocPrint(allocator, "key_{}", .{i});
        defer allocator.free(key);
        filter_bank.add(key);
    }
    
    // Verify keys can be found
    var found_count: usize = 0;
    for (0..1000) |i| {
        const key = try std.fmt.allocPrint(allocator, "key_{}", .{i});
        defer allocator.free(key);
        if (filter_bank.contains(key)) {
            found_count += 1;
        }
    }
    
    // Should find all keys (no false negatives)
    try testing.expect(found_count == 1000);
    
    const stats = filter_bank.getStats();
    try testing.expect(stats.total_entries == 1000);
}

test "BloomFilter optimal parameters" {
    const testing = std.testing;
    
    const params = calculateOptimalParams(1_000_000, 0.01);
    
    // Should have reasonable parameters
    try testing.expect(params.size_bits > 1000);
    try testing.expect(params.hash_functions >= 1 and params.hash_functions <= 16);
    
    print("Optimal params for 1M entries, 1% FPR: {} bits, {} hash functions\n", .{ params.size_bits, params.hash_functions });
}

test "BloomFilter performance" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    try benchmarkBloomFilter(allocator, 10000);
}