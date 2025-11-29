const std = @import("std");
const print = std.debug.print;
const time = std.time;

const ifr = @import("ifr");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    print("🏁 HyperMesh IFR Performance Benchmarks\n");
    print("=========================================\n\n");

    try runComprehensiveBenchmarks(allocator);
}

fn runComprehensiveBenchmarks(allocator: std.mem.Allocator) !void {
    print("🚀 Running comprehensive IFR benchmarks...\n\n");

    // System info
    print("📊 System Information:\n");
    print("   Platform: {s}\n", .{@tagName(std.Target.current.os.tag)});
    print("   Architecture: {s}\n", .{@tagName(std.Target.current.cpu.arch)});
    print("   Build mode: {s}\n", .{@tagName(std.builtin.mode)});
    print("\n");

    // Individual component benchmarks
    try benchmarkExactMatcher(allocator);
    try benchmarkBloomFilter(allocator);
    try benchmarkFlowCache(allocator);
    try benchmarkUnixSocket(allocator);
    
    // Integrated system benchmark
    try benchmarkIntegratedSystem(allocator);
    
    print("🏆 All benchmarks completed successfully!\n");
}

fn benchmarkExactMatcher(allocator: std.mem.Allocator) !void {
    print("1️⃣ === Exact Matcher Benchmark ===\n");
    
    const test_sizes = [_]usize{ 10_000, 100_000, 1_000_000 };
    
    for (test_sizes) |size| {
        print("   Testing with {} entries...\n", .{size});
        
        const matcher = try ifr.ExactMatcher.init(allocator, .{
            .max_entries = size * 2,
            .hash_algorithm = .Blake3,
        });
        defer matcher.deinit();
        
        // Prepare test data
        var test_keys = try allocator.alloc([]u8, size);
        defer {
            for (test_keys) |key| {
                allocator.free(key);
            }
            allocator.free(test_keys);
        }
        
        var rng = std.rand.DefaultPrng.init(@intCast(time.timestamp()));
        for (test_keys, 0..) |*key, i| {
            key.* = try std.fmt.allocPrint(allocator, "key_{}_rand_{}", .{ i, rng.random().int(u64) });
        }
        
        // Benchmark insertions
        const insert_start = time.nanoTimestamp();
        for (test_keys, 0..) |key, i| {
            const flow = ifr.FlowRecord{
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
        
        // Benchmark lookups
        const lookup_start = time.nanoTimestamp();
        var found_count: usize = 0;
        for (test_keys) |key| {
            if (matcher.find(key)) |_| {
                found_count += 1;
            }
        }
        const lookup_end = time.nanoTimestamp();
        
        const insert_duration = insert_end - insert_start;
        const lookup_duration = lookup_end - lookup_start;
        
        const insert_ns_per_op = @as(f64, @floatFromInt(insert_duration)) / @as(f64, @floatFromInt(size));
        const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(size));
        const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
        
        const stats = matcher.getStats();
        
        print("     Insert: {:.2} ns/op\n", .{insert_ns_per_op});
        print("     Lookup: {:.2} ns/op, {:.0} ops/sec\n", .{ lookup_ns_per_op, lookup_ops_per_sec });
        print("     Found: {}/{} ({:.1}%)\n", .{ found_count, size, @as(f64, @floatFromInt(found_count * 100)) / @as(f64, @floatFromInt(size)) });
        print("     Load factor: {:.3}, Max PSL: {}\n", .{ stats.load_factor, stats.max_psl });
        
        // Check performance targets
        const lookup_ms_per_op = lookup_ns_per_op / 1_000_000.0;
        if (lookup_ms_per_op <= 0.1) {
            print("     ✅ Latency target met: {:.3}ms\n", .{lookup_ms_per_op});
        } else {
            print("     ❌ Latency target missed: {:.3}ms (target <0.1ms)\n", .{lookup_ms_per_op});
        }
        
        if (lookup_ops_per_sec >= 10_000_000) {
            print("     ✅ Throughput target met: {:.0} ops/sec\n", .{lookup_ops_per_sec});
        } else {
            print("     ❌ Throughput target missed: {:.0} ops/sec (target >10M)\n", .{lookup_ops_per_sec});
        }
        
        print("\n");
    }
    
    print("\n");
}

fn benchmarkBloomFilter(allocator: std.mem.Allocator) !void {
    print("2️⃣ === Bloom Filter Benchmark ===\n");
    
    const test_configs = [_]struct {
        entries: usize,
        fpr: f64,
    }{
        .{ .entries = 100_000, .fpr = 0.01 },
        .{ .entries = 1_000_000, .fpr = 0.01 },
        .{ .entries = 1_000_000, .fpr = 0.001 },
    };
    
    for (test_configs) |config| {
        print("   Testing {} entries, {:.3}% FPR...\n", .{ config.entries, config.fpr * 100.0 });
        
        const filter_bank = try ifr.BloomFilterBank.init(allocator, .{
            .false_positive_rate = config.fpr,
            .expected_entries = config.entries,
            .hash_functions = 3,
            .max_filters = 4,
        });
        defer filter_bank.deinit();
        
        // Prepare test data
        var test_keys = try allocator.alloc([]u8, config.entries);
        defer {
            for (test_keys) |key| {
                allocator.free(key);
            }
            allocator.free(test_keys);
        }
        
        var rng = std.rand.DefaultPrng.init(@intCast(time.timestamp()));
        for (test_keys, 0..) |*key, i| {
            key.* = try std.fmt.allocPrint(allocator, "bloom_{}_rand_{}", .{ i, rng.random().int(u64) });
        }
        
        // Benchmark additions
        const add_start = time.nanoTimestamp();
        for (test_keys) |key| {
            filter_bank.add(key);
        }
        const add_end = time.nanoTimestamp();
        
        // Benchmark lookups (existing keys)
        const lookup_start = time.nanoTimestamp();
        var hit_count: usize = 0;
        for (test_keys) |key| {
            if (filter_bank.contains(key)) {
                hit_count += 1;
            }
        }
        const lookup_end = time.nanoTimestamp();
        
        // Test false positives
        var false_positives: usize = 0;
        const false_positive_tests = config.entries / 10;
        for (0..false_positive_tests) |i| {
            const nonexistent_key = try std.fmt.allocPrint(allocator, "nonexistent_{}", .{i});
            defer allocator.free(nonexistent_key);
            
            if (filter_bank.contains(nonexistent_key)) {
                false_positives += 1;
            }
        }
        
        const add_duration = add_end - add_start;
        const lookup_duration = lookup_end - lookup_start;
        
        const add_ns_per_op = @as(f64, @floatFromInt(add_duration)) / @as(f64, @floatFromInt(config.entries));
        const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(config.entries));
        const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
        
        const stats = filter_bank.getStats();
        const actual_fpr = @as(f64, @floatFromInt(false_positives)) / @as(f64, @floatFromInt(false_positive_tests));
        
        print("     Add: {:.2} ns/op\n", .{add_ns_per_op});
        print("     Lookup: {:.2} ns/op, {:.0} ops/sec\n", .{ lookup_ns_per_op, lookup_ops_per_sec });
        print("     Hit rate: {:.1}% ({}/{})\n", .{ 
            @as(f64, @floatFromInt(hit_count * 100)) / @as(f64, @floatFromInt(config.entries)), 
            hit_count, 
            config.entries 
        });
        print("     False positive rate: {:.3}% (target: {:.3}%)\n", .{ actual_fpr * 100.0, config.fpr * 100.0 });
        print("     Memory usage: {:.2} MB\n", .{ @as(f64, @floatFromInt(stats.memory_usage_bytes)) / (1024.0 * 1024.0) });
        
        if (actual_fpr <= config.fpr * 2.0) {
            print("     ✅ False positive rate within target\n");
        } else {
            print("     ❌ False positive rate exceeds target\n");
        }
        
        print("\n");
    }
    
    print("\n");
}

fn benchmarkFlowCache(allocator: std.mem.Allocator) !void {
    print("3️⃣ === Flow Cache Benchmark ===\n");
    
    const test_configs = [_]struct {
        entries: usize,
        memory_mb: usize,
        strategy: ifr.FlowCache.FlowCacheConfig.EvictionStrategy,
    }{
        .{ .entries = 100_000, .memory_mb = 50, .strategy = .LRU },
        .{ .entries = 500_000, .memory_mb = 100, .strategy = .LRU },
        .{ .entries = 100_000, .memory_mb = 50, .strategy = .LFU },
    };
    
    for (test_configs) |config| {
        print("   Testing {} entries, {}MB, {} eviction...\n", .{ config.entries, config.memory_mb, config.strategy });
        
        const cache = try ifr.FlowCache.init(allocator, .{
            .max_entries = config.entries,
            .max_memory = config.memory_mb * 1024 * 1024,
            .eviction_strategy = config.strategy,
        });
        defer cache.deinit();
        
        // Prepare test data
        var test_keys = try allocator.alloc([]u8, config.entries);
        defer {
            for (test_keys) |key| {
                allocator.free(key);
            }
            allocator.free(test_keys);
        }
        
        var rng = std.rand.DefaultPrng.init(@intCast(time.timestamp()));
        for (test_keys, 0..) |*key, i| {
            key.* = try std.fmt.allocPrint(allocator, "cache_{}_rand_{}", .{ i, rng.random().int(u64) });
        }
        
        // Benchmark insertions
        const insert_start = time.nanoTimestamp();
        for (test_keys, 0..) |key, i| {
            const flow = ifr.FlowRecord{
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
        
        // Benchmark lookups (cache hits)
        const lookup_start = time.nanoTimestamp();
        var hit_count: usize = 0;
        for (test_keys) |key| {
            if (cache.get(key)) |_| {
                hit_count += 1;
            }
        }
        const lookup_end = time.nanoTimestamp();
        
        const insert_duration = insert_end - insert_start;
        const lookup_duration = lookup_end - lookup_start;
        
        const insert_ns_per_op = @as(f64, @floatFromInt(insert_duration)) / @as(f64, @floatFromInt(config.entries));
        const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(config.entries));
        const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
        
        const stats = cache.getStats();
        
        print("     Insert: {:.2} ns/op\n", .{insert_ns_per_op});
        print("     Lookup: {:.2} ns/op, {:.0} ops/sec\n", .{ lookup_ns_per_op, lookup_ops_per_sec });
        print("     Hit rate: {:.1}% ({}/{})\n", .{ stats.hit_rate * 100.0, hit_count, config.entries });
        print("     Memory: {:.2}/{:.2} MB\n", .{
            @as(f64, @floatFromInt(stats.memory_used)) / (1024.0 * 1024.0),
            @as(f64, @floatFromInt(stats.max_memory)) / (1024.0 * 1024.0),
        });
        print("     Evictions: {}\n", .{stats.eviction_count});
        
        if (lookup_ns_per_op <= 100.0) {
            print("     ✅ Cache latency target met: {:.2} ns\n", .{lookup_ns_per_op});
        } else {
            print("     ❌ Cache latency target missed: {:.2} ns (target <100ns)\n", .{lookup_ns_per_op});
        }
        
        print("\n");
    }
    
    print("\n");
}

fn benchmarkUnixSocket(allocator: std.mem.Allocator) !void {
    print("4️⃣ === Unix Socket Benchmark ===\n");
    
    print("   Creating Unix socket server...\n");
    
    const server = try ifr.UnixSocketServer.init(allocator, .{
        .socket_path = "/tmp/hypermesh_bench/",
        .max_connections = 100,
        .buffer_size = 64 * 1024,
        .timeout_ms = 1000,
        .worker_threads = 2,
    });
    defer server.deinit();
    
    // Create socket directory
    std.fs.makeDirAbsolute("/tmp/hypermesh_bench/") catch {};
    defer {
        var dir = std.fs.openDirAbsolute("/tmp/hypermesh_bench/", .{}) catch return;
        dir.deleteTree(".") catch {};
        dir.close();
    }
    
    // Measure socket server startup time
    const startup_start = time.nanoTimestamp();
    try server.start();
    const startup_end = time.nanoTimestamp();
    defer server.stop() catch {};
    
    const startup_duration = startup_end - startup_start;
    const startup_ms = @as(f64, @floatFromInt(startup_duration)) / 1_000_000.0;
    
    print("     Startup time: {:.2}ms\n", .{startup_ms});
    
    // Allow server to initialize
    std.time.sleep(100_000_000); // 100ms
    
    const stats = server.getStats();
    print("     Server running: {}\n", .{stats.is_running});
    print("     Worker threads: {}\n", .{stats.worker_threads});
    print("     Active connections: {}\n", .{stats.active_connections});
    
    if (startup_ms <= 100.0) {
        print("     ✅ Startup time target met: {:.2}ms (target <100ms)\n", .{startup_ms});
    } else {
        print("     ❌ Startup time target missed: {:.2}ms (target <100ms)\n", .{startup_ms});
    }
    
    print("\n\n");
}

fn benchmarkIntegratedSystem(allocator: std.mem.Allocator) !void {
    print("5️⃣ === Integrated System Benchmark ===\n");
    
    print("   Creating full IFR registry...\n");
    
    const registry = try ifr.IFRRegistry.init(allocator);
    defer registry.deinit();
    
    const startup_start = time.nanoTimestamp();
    try registry.start();
    const startup_end = time.nanoTimestamp();
    defer registry.stop() catch {};
    
    const startup_duration = startup_end - startup_start;
    const startup_ms = @as(f64, @floatFromInt(startup_duration)) / 1_000_000.0;
    
    print("     Full system startup: {:.2}ms\n", .{startup_ms});
    
    // Allow system to initialize
    std.time.sleep(500_000_000); // 500ms
    
    // Test mixed workload
    const num_flows = 50_000;
    print("   Testing mixed workload with {} flows...\n", .{num_flows});
    
    // Register flows
    const register_start = time.nanoTimestamp();
    for (0..num_flows) |i| {
        const key = try std.fmt.allocPrint(allocator, "integrated_flow_{}", .{i});
        defer allocator.free(key);
        
        var flow_key: [32]u8 = undefined;
        @memcpy(flow_key[0..key.len], key);
        if (key.len < 32) {
            @memset(flow_key[key.len..], 0);
        }
        
        const flow = ifr.FlowRecord{
            .key = flow_key,
            .component_id = @as(u32, @intCast(i % 7)),
            .flow_type = @enumFromInt(@as(u8, @intCast(i % 6))),
            .timestamp = @intCast(time.nanoTimestamp()),
            .size = @as(u32, @intCast(512 + (i % 2048))),
            .priority = @as(u8, @intCast(i % 8)),
        };
        
        try registry.registerFlow(flow);
    }
    const register_end = time.nanoTimestamp();
    
    // Lookup flows
    const lookup_start = time.nanoTimestamp();
    var found_count: usize = 0;
    for (0..num_flows) |i| {
        const key = try std.fmt.allocPrint(allocator, "integrated_flow_{}", .{i});
        defer allocator.free(key);
        
        if (registry.lookup(key)) |_| {
            found_count += 1;
        }
    }
    const lookup_end = time.nanoTimestamp();
    
    // Test coordination
    const coord_start = time.nanoTimestamp();
    const coord_iterations = 1000;
    for (0..coord_iterations) |_| {
        const test_message = "coordination_test_message";
        registry.coordinateLocal(.Transport, test_message) catch {};
    }
    const coord_end = time.nanoTimestamp();
    
    const register_duration = register_end - register_start;
    const lookup_duration = lookup_end - lookup_start;
    const coord_duration = coord_end - coord_start;
    
    const register_ns_per_op = @as(f64, @floatFromInt(register_duration)) / @as(f64, @floatFromInt(num_flows));
    const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(num_flows));
    const coord_ns_per_op = @as(f64, @floatFromInt(coord_duration)) / @as(f64, @floatFromInt(coord_iterations));
    
    const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
    const register_ms_per_op = register_ns_per_op / 1_000_000.0;
    const lookup_ms_per_op = lookup_ns_per_op / 1_000_000.0;
    const coord_us_per_op = coord_ns_per_op / 1_000.0;
    
    print("     Registration: {:.3}ms per op\n", .{register_ms_per_op});
    print("     Lookup: {:.3}ms per op, {:.0} ops/sec\n", .{ lookup_ms_per_op, lookup_ops_per_sec });
    print("     Coordination: {:.1}µs per op\n", .{coord_us_per_op});
    print("     Found flows: {}/{} ({:.1}%)\n", .{
        found_count,
        num_flows,
        @as(f64, @floatFromInt(found_count * 100)) / @as(f64, @floatFromInt(num_flows)),
    });
    
    const final_stats = registry.getPerformanceStats();
    print("     Cache hit rate: {:.1}%\n", .{final_stats.cache_hit_rate * 100.0});
    print("     System health: {}\n", .{registry.healthCheck()});
    
    // Overall performance assessment
    print("\n   🎯 Integrated Performance Assessment:\n");
    
    if (lookup_ms_per_op <= 0.1) {
        print("     ✅ Lookup latency: {:.3}ms (target <0.1ms)\n", .{lookup_ms_per_op});
    } else {
        print("     ❌ Lookup latency: {:.3}ms (target <0.1ms)\n", .{lookup_ms_per_op});
    }
    
    if (lookup_ops_per_sec >= 10_000_000) {
        print("     ✅ Throughput: {:.0} ops/sec (target >10M)\n", .{lookup_ops_per_sec});
    } else {
        print("     ❌ Throughput: {:.0} ops/sec (target >10M)\n", .{lookup_ops_per_sec});
    }
    
    if (coord_us_per_op <= 50.0) {
        print("     ✅ Coordination: {:.1}µs (target <50µs)\n", .{coord_us_per_op});
    } else {
        print("     ❌ Coordination: {:.1}µs (target <50µs)\n", .{coord_us_per_op});
    }
    
    const memory_mb = @as(f64, @floatFromInt(final_stats.memory_usage_bytes)) / (1024.0 * 1024.0);
    if (memory_mb <= 10.0) {
        print("     ✅ Memory: {:.1}MB (target <10MB)\n", .{memory_mb});
    } else {
        print("     ❌ Memory: {:.1}MB (target <10MB)\n", .{memory_mb});
    }
    
    print("\n");
}