const std = @import("std");
const print = std.debug.print;
const Allocator = std.mem.Allocator;
const ArrayList = std.ArrayList;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const time = std.time;

// Performance metrics collection for IFR
pub const IFRMetrics = struct {
    allocator: Allocator,
    mutex: Mutex,
    
    // Lookup metrics
    lookup_latency: Histogram,
    lookup_count: u64,
    cache_hits: u64,
    cache_misses: u64,
    bloom_filter_rejects: u64,
    
    // Registration metrics
    registration_latency: Histogram,
    registration_count: u64,
    
    // Coordination metrics
    coordination_latency: Histogram,
    coordination_messages: u64,
    
    // System metrics
    memory_usage: u64,
    active_flows: u64,
    start_time: u64,
    
    // Collection thread
    collection_thread: ?Thread,
    collection_running: bool,
    
    const Self = @This();
    
    pub fn init(allocator: Allocator) !*Self {
        const self = try allocator.create(Self);
        errdefer allocator.destroy(self);
        
        const lookup_latency = try Histogram.init(allocator, "lookup_latency_ms");
        errdefer lookup_latency.deinit();
        
        const registration_latency = try Histogram.init(allocator, "registration_latency_ms");
        errdefer registration_latency.deinit();
        
        const coordination_latency = try Histogram.init(allocator, "coordination_latency_us");
        errdefer coordination_latency.deinit();
        
        self.* = Self{
            .allocator = allocator,
            .mutex = Mutex{},
            .lookup_latency = lookup_latency,
            .lookup_count = 0,
            .cache_hits = 0,
            .cache_misses = 0,
            .bloom_filter_rejects = 0,
            .registration_latency = registration_latency,
            .registration_count = 0,
            .coordination_latency = coordination_latency,
            .coordination_messages = 0,
            .memory_usage = 0,
            .active_flows = 0,
            .start_time = @intCast(time.nanoTimestamp()),
            .collection_thread = null,
            .collection_running = false,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self) void {
        if (self.collection_running) {
            self.stopCollection() catch {};
        }
        
        self.lookup_latency.deinit();
        self.registration_latency.deinit();
        self.coordination_latency.deinit();
        self.allocator.destroy(self);
    }
    
    pub fn startCollection(self: *Self) !void {
        if (self.collection_running) return;
        
        self.collection_running = true;
        self.collection_thread = try Thread.spawn(.{}, collectionThread, .{self});
        
        print("[IFRMetrics] Started metrics collection\n");
    }
    
    pub fn stopCollection(self: *Self) !void {
        if (!self.collection_running) return;
        
        self.collection_running = false;
        
        if (self.collection_thread) |thread| {
            thread.join();
            self.collection_thread = null;
        }
        
        print("[IFRMetrics] Stopped metrics collection\n");
    }
    
    // Record lookup latency in milliseconds
    pub fn recordLookupLatency(self: *Self, latency_ms: f64) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        self.lookup_latency.observe(latency_ms);
        self.lookup_count += 1;
    }
    
    // Record registration latency in milliseconds
    pub fn recordRegistrationLatency(self: *Self, latency_ms: f64) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        self.registration_latency.observe(latency_ms);
        self.registration_count += 1;
    }
    
    // Record coordination latency in microseconds
    pub fn recordCoordinationLatency(self: *Self, latency_us: f64) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        self.coordination_latency.observe(latency_us);
    }
    
    pub fn incrementCacheHits(self: *Self) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.cache_hits += 1;
    }
    
    pub fn incrementCacheMisses(self: *Self) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.cache_misses += 1;
    }
    
    pub fn incrementBloomFilterRejects(self: *Self) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.bloom_filter_rejects += 1;
    }
    
    pub fn incrementFlowRegistrations(self: *Self) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.active_flows += 1;
    }
    
    pub fn incrementCoordinationMessages(self: *Self) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.coordination_messages += 1;
    }
    
    pub fn updateMemoryUsage(self: *Self, memory_bytes: u64) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.memory_usage = memory_bytes;
    }
    
    // Get comprehensive performance statistics
    pub fn getStats(self: *Self) PerformanceStats {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        const now = @as(u64, @intCast(time.nanoTimestamp()));
        const uptime_seconds = @as(f64, @floatFromInt(now - self.start_time)) / 1_000_000_000.0;
        
        const cache_hit_rate = if (self.cache_hits + self.cache_misses > 0)
            @as(f64, @floatFromInt(self.cache_hits)) / @as(f64, @floatFromInt(self.cache_hits + self.cache_misses))
        else
            0.0;
        
        const lookups_per_second = if (uptime_seconds > 0)
            @as(f64, @floatFromInt(self.lookup_count)) / uptime_seconds
        else
            0.0;
        
        const registrations_per_second = if (uptime_seconds > 0)
            @as(f64, @floatFromInt(self.registration_count)) / uptime_seconds
        else
            0.0;
        
        const coordination_per_second = if (uptime_seconds > 0)
            @as(f64, @floatFromInt(self.coordination_messages)) / uptime_seconds
        else
            0.0;
        
        return PerformanceStats{
            .uptime_seconds = uptime_seconds,
            .lookup_count = self.lookup_count,
            .lookups_per_second = lookups_per_second,
            .avg_lookup_latency_ms = self.lookup_latency.mean(),
            .p50_lookup_latency_ms = self.lookup_latency.percentile(0.5),
            .p95_lookup_latency_ms = self.lookup_latency.percentile(0.95),
            .p99_lookup_latency_ms = self.lookup_latency.percentile(0.99),
            .cache_hit_rate = cache_hit_rate,
            .cache_hits = self.cache_hits,
            .cache_misses = self.cache_misses,
            .bloom_filter_rejects = self.bloom_filter_rejects,
            .registration_count = self.registration_count,
            .registrations_per_second = registrations_per_second,
            .avg_registration_latency_ms = self.registration_latency.mean(),
            .coordination_messages = self.coordination_messages,
            .coordination_per_second = coordination_per_second,
            .avg_coordination_latency_us = self.coordination_latency.mean(),
            .memory_usage_bytes = self.memory_usage,
            .active_flows = self.active_flows,
        };
    }
    
    pub const PerformanceStats = struct {
        uptime_seconds: f64,
        lookup_count: u64,
        lookups_per_second: f64,
        avg_lookup_latency_ms: f64,
        p50_lookup_latency_ms: f64,
        p95_lookup_latency_ms: f64,
        p99_lookup_latency_ms: f64,
        cache_hit_rate: f64,
        cache_hits: u64,
        cache_misses: u64,
        bloom_filter_rejects: u64,
        registration_count: u64,
        registrations_per_second: f64,
        avg_registration_latency_ms: f64,
        coordination_messages: u64,
        coordination_per_second: f64,
        avg_coordination_latency_us: f64,
        memory_usage_bytes: u64,
        active_flows: u64,
    };
    
    // Print performance report
    pub fn printReport(self: *Self) void {
        const stats = self.getStats();
        
        print("\n=== IFR Performance Report ===\n");
        print("Uptime: {:.2}s\n", .{stats.uptime_seconds});
        print("\nLookup Performance:\n");
        print("  Total lookups: {}\n", .{stats.lookup_count});
        print("  Lookups/sec: {:.0}\n", .{stats.lookups_per_second});
        print("  Avg latency: {:.3}ms\n", .{stats.avg_lookup_latency_ms});
        print("  P50 latency: {:.3}ms\n", .{stats.p50_lookup_latency_ms});
        print("  P95 latency: {:.3}ms\n", .{stats.p95_lookup_latency_ms});
        print("  P99 latency: {:.3}ms\n", .{stats.p99_lookup_latency_ms});
        
        print("\nCache Performance:\n");
        print("  Hit rate: {:.1}%\n", .{stats.cache_hit_rate * 100.0});
        print("  Cache hits: {}\n", .{stats.cache_hits});
        print("  Cache misses: {}\n", .{stats.cache_misses});
        print("  Bloom filter rejects: {}\n", .{stats.bloom_filter_rejects});
        
        print("\nRegistration Performance:\n");
        print("  Total registrations: {}\n", .{stats.registration_count});
        print("  Registrations/sec: {:.1}\n", .{stats.registrations_per_second});
        print("  Avg latency: {:.3}ms\n", .{stats.avg_registration_latency_ms});
        
        print("\nCoordination Performance:\n");
        print("  Total messages: {}\n", .{stats.coordination_messages});
        print("  Messages/sec: {:.1}\n", .{stats.coordination_per_second});
        print("  Avg latency: {:.1}µs\n", .{stats.avg_coordination_latency_us});
        
        print("\nSystem Status:\n");
        print("  Memory usage: {:.2}MB\n", .{@as(f64, @floatFromInt(stats.memory_usage_bytes)) / (1024.0 * 1024.0)});
        print("  Active flows: {}\n", .{stats.active_flows});
        
        print("\nPerformance Targets:\n");
        
        // Check lookup target (<0.1ms)
        if (stats.avg_lookup_latency_ms > 0.1) {
            print("  ❌ Lookup latency: {:.3}ms (target: <0.1ms)\n", .{stats.avg_lookup_latency_ms});
        } else {
            print("  ✅ Lookup latency: {:.3}ms (target: <0.1ms)\n", .{stats.avg_lookup_latency_ms});
        }
        
        // Check throughput target (>10M ops/sec)
        if (stats.lookups_per_second < 10_000_000) {
            print("  ❌ Throughput: {:.0} ops/sec (target: >10M ops/sec)\n", .{stats.lookups_per_second});
        } else {
            print("  ✅ Throughput: {:.0} ops/sec (target: >10M ops/sec)\n", .{stats.lookups_per_second});
        }
        
        // Check coordination latency target (<50µs)
        if (stats.avg_coordination_latency_us > 50.0) {
            print("  ❌ Coordination latency: {:.1}µs (target: <50µs)\n", .{stats.avg_coordination_latency_us});
        } else {
            print("  ✅ Coordination latency: {:.1}µs (target: <50µs)\n", .{stats.avg_coordination_latency_us});
        }
        
        // Check memory usage target (<10MB per node)
        if (stats.memory_usage_bytes > 10 * 1024 * 1024) {
            print("  ❌ Memory usage: {:.2}MB (target: <10MB per node)\n", .{@as(f64, @floatFromInt(stats.memory_usage_bytes)) / (1024.0 * 1024.0)});
        } else {
            print("  ✅ Memory usage: {:.2}MB (target: <10MB per node)\n", .{@as(f64, @floatFromInt(stats.memory_usage_bytes)) / (1024.0 * 1024.0)});
        }
        
        print("================================\n\n");
    }
    
    // Export metrics in Prometheus format
    pub fn exportPrometheus(self: *Self, writer: anytype) !void {
        const stats = self.getStats();
        
        try writer.print("# HELP ifr_lookup_total Total number of flow lookups\n");
        try writer.print("# TYPE ifr_lookup_total counter\n");
        try writer.print("ifr_lookup_total {}\n", .{stats.lookup_count});
        
        try writer.print("# HELP ifr_lookup_latency_seconds Lookup latency in seconds\n");
        try writer.print("# TYPE ifr_lookup_latency_seconds histogram\n");
        try self.lookup_latency.exportPrometheus(writer, "ifr_lookup_latency_seconds");
        
        try writer.print("# HELP ifr_cache_hit_rate Cache hit rate\n");
        try writer.print("# TYPE ifr_cache_hit_rate gauge\n");
        try writer.print("ifr_cache_hit_rate {:.6}\n", .{stats.cache_hit_rate});
        
        try writer.print("# HELP ifr_coordination_latency_seconds Coordination latency in seconds\n");
        try writer.print("# TYPE ifr_coordination_latency_seconds histogram\n");
        try self.coordination_latency.exportPrometheus(writer, "ifr_coordination_latency_seconds");
        
        try writer.print("# HELP ifr_memory_usage_bytes Memory usage in bytes\n");
        try writer.print("# TYPE ifr_memory_usage_bytes gauge\n");
        try writer.print("ifr_memory_usage_bytes {}\n", .{stats.memory_usage_bytes});
        
        try writer.print("# HELP ifr_active_flows Active flow records\n");
        try writer.print("# TYPE ifr_active_flows gauge\n");
        try writer.print("ifr_active_flows {}\n", .{stats.active_flows});
    }
    
    // Background metrics collection thread
    fn collectionThread(self: *Self) void {
        print("[IFRMetrics] Collection thread started\n");
        
        while (self.collection_running) {
            // Collect system metrics every 5 seconds
            std.time.sleep(5_000_000_000); // 5 seconds
            
            if (!self.collection_running) break;
            
            // Update system metrics
            // TODO: Collect actual memory usage from system
            
            // Print periodic report (every 30 seconds)
            const now = @as(u64, @intCast(time.timestamp()));
            if (now % 30 == 0) {
                self.printReport();
            }
        }
        
        print("[IFRMetrics] Collection thread stopped\n");
    }
};

// High-performance histogram for latency measurements
pub const Histogram = struct {
    allocator: Allocator,
    name: []const u8,
    buckets: []f64,
    counts: []u64,
    total_count: u64,
    sum: f64,
    
    const Self = @This();
    
    const DEFAULT_BUCKETS = [_]f64{
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0
    };
    
    pub fn init(allocator: Allocator, name: []const u8) !Self {
        const buckets = try allocator.dupe(f64, &DEFAULT_BUCKETS);
        const counts = try allocator.alloc(u64, buckets.len);
        @memset(counts, 0);
        
        return Self{
            .allocator = allocator,
            .name = try allocator.dupe(u8, name),
            .buckets = buckets,
            .counts = counts,
            .total_count = 0,
            .sum = 0.0,
        };
    }
    
    pub fn deinit(self: *Self) void {
        self.allocator.free(self.name);
        self.allocator.free(self.buckets);
        self.allocator.free(self.counts);
    }
    
    pub fn observe(self: *Self, value: f64) void {
        self.total_count += 1;
        self.sum += value;
        
        // Find appropriate bucket
        for (self.buckets, 0..) |bucket, i| {
            if (value <= bucket) {
                self.counts[i] += 1;
            }
        }
    }
    
    pub fn mean(self: *Self) f64 {
        if (self.total_count == 0) return 0.0;
        return self.sum / @as(f64, @floatFromInt(self.total_count));
    }
    
    pub fn percentile(self: *Self, p: f64) f64 {
        if (self.total_count == 0) return 0.0;
        
        const target_count = @as(u64, @intFromFloat(@as(f64, @floatFromInt(self.total_count)) * p));
        var cumulative_count: u64 = 0;
        
        for (self.buckets, 0..) |bucket, i| {
            cumulative_count += self.counts[i];
            if (cumulative_count >= target_count) {
                return bucket;
            }
        }
        
        return self.buckets[self.buckets.len - 1];
    }
    
    pub fn exportPrometheus(self: *Self, writer: anytype, metric_name: []const u8) !void {
        var cumulative_count: u64 = 0;
        
        for (self.buckets, 0..) |bucket, i| {
            cumulative_count += self.counts[i];
            try writer.print("{}{{le=\"{}\"}} {}\n", .{ metric_name, bucket, cumulative_count });
        }
        
        try writer.print("{}{{le=\"+Inf\"}} {}\n", .{ metric_name, self.total_count });
        try writer.print("{}_sum {}\n", .{ metric_name, self.sum });
        try writer.print("{}_count {}\n", .{ metric_name, self.total_count });
    }
};

// Unit tests
test "IFRMetrics basic operations" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const metrics = try IFRMetrics.init(allocator);
    defer metrics.deinit();
    
    // Record some metrics
    metrics.recordLookupLatency(0.05); // 0.05ms
    metrics.recordLookupLatency(0.1);  // 0.1ms
    metrics.incrementCacheHits();
    metrics.incrementCacheMisses();
    
    const stats = metrics.getStats();
    
    try testing.expect(stats.lookup_count == 2);
    try testing.expect(stats.cache_hits == 1);
    try testing.expect(stats.cache_misses == 1);
    try testing.expect(stats.cache_hit_rate == 0.5);
    try testing.expect(stats.avg_lookup_latency_ms > 0.0);
}

test "Histogram percentiles" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    var histogram = try Histogram.init(allocator, "test_histogram");
    defer histogram.deinit();
    
    // Add some values
    for (0..100) |i| {
        histogram.observe(@as(f64, @floatFromInt(i)) / 100.0);
    }
    
    // Test percentiles
    const p50 = histogram.percentile(0.5);
    const p95 = histogram.percentile(0.95);
    const p99 = histogram.percentile(0.99);
    
    try testing.expect(p50 > 0.4 and p50 < 0.6);
    try testing.expect(p95 > 0.9);
    try testing.expect(p99 > 0.98);
    
    try testing.expect(histogram.mean() > 0.4 and histogram.mean() < 0.6);
}