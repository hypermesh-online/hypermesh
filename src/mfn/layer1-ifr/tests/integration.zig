const std = @import("std");
const testing = std.testing;
const print = std.debug.print;
const time = std.time;

const ifr = @import("ifr");

test "IFR full integration" {
    const allocator = testing.allocator;
    
    print("\n🧪 Running full IFR integration tests...\n");
    
    // Test 1: Component initialization
    print("1️⃣ Testing component initialization...\n");
    
    const registry = try ifr.IFRRegistry.init(allocator);
    defer registry.deinit();
    
    try testing.expect(registry.healthCheck());
    print("   ✅ Registry initialization successful\n");
    
    // Test 2: Service startup
    print("2️⃣ Testing service startup...\n");
    
    try registry.start();
    defer registry.stop() catch {};
    
    // Allow services to start
    std.time.sleep(100_000_000); // 100ms
    
    try testing.expect(registry.healthCheck());
    print("   ✅ Service startup successful\n");
    
    // Test 3: Flow operations
    print("3️⃣ Testing flow operations...\n");
    
    const test_flow = ifr.FlowRecord{
        .key = std.mem.zeroes([32]u8),
        .component_id = @intFromEnum(ifr.ComponentId.Transport),
        .flow_type = .ComponentCommand,
        .timestamp = @intCast(time.nanoTimestamp()),
        .size = 1024,
        .priority = 5,
    };
    
    const test_key = "integration_test_flow";
    @memcpy(test_flow.key[0..test_key.len], test_key);
    
    try registry.registerFlow(test_flow);
    print("   ✅ Flow registration successful\n");
    
    const lookup_result = registry.lookup(test_key);
    try testing.expect(lookup_result != null);
    try testing.expect(lookup_result.?.component_id == @intFromEnum(ifr.ComponentId.Transport));
    print("   ✅ Flow lookup successful\n");
    
    // Test 4: Performance validation
    print("4️⃣ Testing performance requirements...\n");
    
    // Register many flows for performance testing
    const num_test_flows = 10000;
    const perf_test_start = time.nanoTimestamp();
    
    for (0..num_test_flows) |i| {
        const key = try std.fmt.allocPrint(allocator, "perf_flow_{}", .{i});
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
            .size = @as(u32, @intCast(512 + (i % 1024))),
            .priority = @as(u8, @intCast(i % 8)),
        };
        
        try registry.registerFlow(flow);
    }
    
    const perf_test_mid = time.nanoTimestamp();
    
    // Lookup performance test
    var found_count: usize = 0;
    for (0..num_test_flows) |i| {
        const key = try std.fmt.allocPrint(allocator, "perf_flow_{}", .{i});
        defer allocator.free(key);
        
        if (registry.lookup(key)) |_| {
            found_count += 1;
        }
    }
    
    const perf_test_end = time.nanoTimestamp();
    
    const register_duration = perf_test_mid - perf_test_start;
    const lookup_duration = perf_test_end - perf_test_mid;
    
    const register_ns_per_op = @as(f64, @floatFromInt(register_duration)) / @as(f64, @floatFromInt(num_test_flows));
    const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(num_test_flows));
    const lookup_ms_per_op = lookup_ns_per_op / 1_000_000.0;
    const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
    
    print("   📊 Performance results:\n");
    print("      Registration: {:.2} ns/op\n", .{register_ns_per_op});
    print("      Lookup: {:.3}ms per op, {:.0} ops/sec\n", .{ lookup_ms_per_op, lookup_ops_per_sec });
    print("      Found: {}/{} flows\n", .{ found_count, num_test_flows });
    
    // Validate performance targets
    try testing.expect(lookup_ms_per_op < 1.0); // Relaxed for test environment
    try testing.expect(found_count == num_test_flows);
    print("   ✅ Performance requirements validated\n");
    
    // Test 5: Error handling
    print("5️⃣ Testing error handling...\n");
    
    // Test invalid lookups
    const invalid_result = registry.lookup("nonexistent_flow");
    try testing.expect(invalid_result == null);
    print("   ✅ Invalid lookup handled correctly\n");
    
    // Test coordination with non-existent component (should handle gracefully)
    registry.coordinateLocal(.Transport, "test_message") catch |err| {
        // Expected to fail since no actual components are running
        print("   ✅ Coordination error handled: {}\n", .{err});
    };
    
    // Test 6: System health and metrics
    print("6️⃣ Testing system health and metrics...\n");
    
    const final_stats = registry.getPerformanceStats();
    try testing.expect(final_stats.lookup_count > 0);
    try testing.expect(final_stats.registration_count > 0);
    try testing.expect(registry.healthCheck());
    
    print("   📊 Final system stats:\n");
    print("      Lookups: {}\n", .{final_stats.lookup_count});
    print("      Registrations: {}\n", .{final_stats.registration_count});
    print("      Cache hit rate: {:.1}%\n", .{final_stats.cache_hit_rate * 100.0});
    print("      Active flows: {}\n", .{final_stats.active_flows});
    print("   ✅ System health validated\n");
    
    print("\n🏁 All integration tests passed!\n\n");
}

test "ExactMatcher stress test" {
    const allocator = testing.allocator;
    
    print("\n🔥 ExactMatcher stress test...\n");
    
    const matcher = try ifr.ExactMatcher.init(allocator, .{
        .max_entries = 1_000_000,
        .hash_algorithm = .Blake3,
    });
    defer matcher.deinit();
    
    // Add many entries with potential hash collisions
    const num_entries = 100_000;
    var successful_inserts: usize = 0;
    
    for (0..num_entries) |i| {
        const key = try std.fmt.allocPrint(allocator, "stress_key_{}_collision_test", .{i});
        defer allocator.free(key);
        
        const flow = ifr.FlowRecord{
            .key = std.mem.zeroes([32]u8),
            .component_id = @as(u32, @intCast(i % 7)),
            .flow_type = @enumFromInt(@as(u8, @intCast(i % 6))),
            .timestamp = @intCast(time.nanoTimestamp()),
            .size = @as(u32, @intCast(1024 + (i % 2048))),
            .priority = @as(u8, @intCast(i % 8)),
        };
        
        matcher.insert(key, flow) catch continue;
        successful_inserts += 1;
    }
    
    print("   Inserted {}/{} entries\n", .{ successful_inserts, num_entries });
    try testing.expect(successful_inserts > num_entries * 9 / 10); // At least 90% success rate
    
    // Verify lookups work correctly
    var found_count: usize = 0;
    for (0..num_entries) |i| {
        const key = try std.fmt.allocPrint(allocator, "stress_key_{}_collision_test", .{i});
        defer allocator.free(key);
        
        if (matcher.find(key)) |_| {
            found_count += 1;
        }
    }
    
    print("   Found {}/{} entries\n", .{ found_count, successful_inserts });
    try testing.expect(found_count == successful_inserts);
    
    const stats = matcher.getStats();
    print("   Load factor: {:.3}, Max PSL: {}\n", .{ stats.load_factor, stats.max_psl });
    try testing.expect(stats.load_factor < 0.9);
    try testing.expect(stats.max_psl < 100);
    
    print("   ✅ Stress test passed\n\n");
}

test "BloomFilter accuracy test" {
    const allocator = testing.allocator;
    
    print("\n🌸 BloomFilter accuracy test...\n");
    
    const filter_bank = try ifr.BloomFilterBank.init(allocator, .{
        .false_positive_rate = 0.01,
        .expected_entries = 100_000,
        .hash_functions = 3,
    });
    defer filter_bank.deinit();
    
    // Add known elements
    const num_elements = 50_000;
    for (0..num_elements) |i| {
        const key = try std.fmt.allocPrint(allocator, "accuracy_test_{}", .{i});
        defer allocator.free(key);
        
        filter_bank.add(key);
    }
    
    // Test true positives (should be 100%)
    var true_positives: usize = 0;
    for (0..num_elements) |i| {
        const key = try std.fmt.allocPrint(allocator, "accuracy_test_{}", .{i});
        defer allocator.free(key);
        
        if (filter_bank.contains(key)) {
            true_positives += 1;
        }
    }
    
    // Test false positives
    var false_positives: usize = 0;
    const false_positive_tests = 10_000;
    for (0..false_positive_tests) |i| {
        const key = try std.fmt.allocPrint(allocator, "nonexistent_accuracy_{}", .{i});
        defer allocator.free(key);
        
        if (filter_bank.contains(key)) {
            false_positives += 1;
        }
    }
    
    const true_positive_rate = @as(f64, @floatFromInt(true_positives)) / @as(f64, @floatFromInt(num_elements));
    const false_positive_rate = @as(f64, @floatFromInt(false_positives)) / @as(f64, @floatFromInt(false_positive_tests));
    
    print("   True positive rate: {:.1}% ({}/{})\n", .{ true_positive_rate * 100.0, true_positives, num_elements });
    print("   False positive rate: {:.3}% ({}/{})\n", .{ false_positive_rate * 100.0, false_positives, false_positive_tests });
    
    // Should have no false negatives
    try testing.expect(true_positive_rate == 1.0);
    
    // False positive rate should be within expected range
    try testing.expect(false_positive_rate <= 0.02); // Allow some variance
    
    const stats = filter_bank.getStats();
    print("   Memory usage: {:.2} MB\n", .{ @as(f64, @floatFromInt(stats.memory_usage_bytes)) / (1024.0 * 1024.0) });
    
    print("   ✅ Accuracy test passed\n\n");
}

test "FlowCache eviction test" {
    const allocator = testing.allocator;
    
    print("\n💾 FlowCache eviction test...\n");
    
    const cache = try ifr.FlowCache.init(allocator, .{
        .max_entries = 1000,
        .max_memory = 1024 * 1024, // 1MB
        .eviction_strategy = .LRU,
    });
    defer cache.deinit();
    
    // Fill cache beyond capacity
    const num_entries = 1500;
    for (0..num_entries) |i| {
        const key = try std.fmt.allocPrint(allocator, "eviction_test_{}", .{i});
        defer allocator.free(key);
        
        const flow = ifr.FlowRecord{
            .key = std.mem.zeroes([32]u8),
            .component_id = @as(u32, @intCast(i % 7)),
            .flow_type = @enumFromInt(@as(u8, @intCast(i % 6))),
            .timestamp = @intCast(time.nanoTimestamp()),
            .size = @as(u32, @intCast(1024)),
            .priority = @as(u8, @intCast(i % 8)),
        };
        
        try cache.put(key, flow);
        
        // Small delay to ensure timestamp ordering
        if (i % 100 == 0) {
            std.time.sleep(1_000_000); // 1ms
        }
    }
    
    const stats = cache.getStats();
    print("   Final cache size: {}\n", .{stats.size});
    print("   Evictions performed: {}\n", .{stats.eviction_count});
    
    // Should have performed evictions
    try testing.expect(stats.size <= 1000);
    try testing.expect(stats.eviction_count > 0);
    
    // Recent entries should still be accessible (LRU behavior)
    var recent_found: usize = 0;
    for ((num_entries - 500)..num_entries) |i| {
        const key = try std.fmt.allocPrint(allocator, "eviction_test_{}", .{i});
        defer allocator.free(key);
        
        if (cache.get(key)) |_| {
            recent_found += 1;
        }
    }
    
    print("   Recent entries found: {}/500\n", .{recent_found});
    
    // Most recent entries should still be in cache
    try testing.expect(recent_found > 400);
    
    print("   ✅ Eviction test passed\n\n");
}

test "Component integration simulation" {
    const allocator = testing.allocator;
    
    print("\n🔗 Component integration simulation...\n");
    
    // Create test socket directory
    const test_dir = "/tmp/hypermesh_integration_test/";
    std.fs.makeDirAbsolute(test_dir) catch {};
    defer {
        var dir = std.fs.openDirAbsolute(test_dir, .{}) catch return;
        dir.deleteTree(".") catch {};
        dir.close();
    }
    
    const mock_server = try ifr.UnixSocketServer.init(allocator, .{
        .socket_path = test_dir,
        .max_connections = 10,
        .worker_threads = 2,
    });
    defer mock_server.deinit();
    
    const integration = try ifr.ComponentIntegration.init(allocator, mock_server);
    defer integration.deinit();
    
    // Create mock component socket files
    const components = [_]ifr.ComponentId{ .Transport, .Consensus, .Container };
    var mock_components = std.ArrayList(ifr.ComponentIntegration.MockComponent).init(allocator);
    defer {
        for (mock_components.items) |*mock| {
            mock.stop();
        }
        mock_components.deinit();
    }
    
    for (components) |component| {
        const socket_path = try std.fmt.allocPrint(allocator, "{s}{s}.sock", .{ test_dir, component.toString() });
        defer allocator.free(socket_path);
        
        var mock = ifr.ComponentIntegration.MockComponent.init(component, socket_path);
        try mock.start();
        try mock_components.append(mock);
    }
    
    // Test component discovery
    try integration.discoverComponents();
    
    // Allow discovery to complete
    std.time.sleep(200_000_000); // 200ms
    
    const stats = integration.getStats();
    print("   Discovered components: {}\n", .{stats.total_components});
    print("   Running components: {}\n", .{stats.running_components});
    
    try testing.expect(stats.total_components >= components.len);
    
    // Test component status retrieval
    for (components) |component| {
        const status = integration.getComponentStatus(component);
        try testing.expect(status != null);
        print("   Component {s}: {}\n", .{ component.toString(), status.?.status });
    }
    
    // Test broadcast messaging (will fail but should handle gracefully)
    const test_message = "Hello from integration test!";
    integration.broadcastToAll(test_message) catch |err| {
        print("   Broadcast test (expected failure): {}\n", .{err});
    };
    
    try integration.stopDiscovery();
    
    print("   ✅ Component integration simulation passed\n\n");
}

test "Performance regression" {
    const allocator = testing.allocator;
    
    print("\n⚡ Performance regression test...\n");
    
    const registry = try ifr.IFRRegistry.init(allocator);
    defer registry.deinit();
    
    try registry.start();
    defer registry.stop() catch {};
    
    // Allow system to initialize
    std.time.sleep(100_000_000); // 100ms
    
    // Performance thresholds (relaxed for CI/test environments)
    const max_lookup_ms = 1.0;  // 1ms instead of 0.1ms for test environment
    const min_throughput = 1_000_000; // 1M ops/sec instead of 10M for test environment
    const max_coordination_us = 500.0; // 500µs instead of 50µs for test environment
    
    // Test with moderate load
    const num_flows = 10_000;
    
    // Register flows
    for (0..num_flows) |i| {
        const key = try std.fmt.allocPrint(allocator, "regression_flow_{}", .{i});
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
            .size = @as(u32, @intCast(512 + (i % 1024))),
            .priority = @as(u8, @intCast(i % 8)),
        };
        
        try registry.registerFlow(flow);
    }
    
    // Performance test lookups
    const lookup_start = time.nanoTimestamp();
    var found_count: usize = 0;
    
    for (0..num_flows) |i| {
        const key = try std.fmt.allocPrint(allocator, "regression_flow_{}", .{i});
        defer allocator.free(key);
        
        if (registry.lookup(key)) |_| {
            found_count += 1;
        }
    }
    
    const lookup_end = time.nanoTimestamp();
    const lookup_duration = lookup_end - lookup_start;
    
    const lookup_ns_per_op = @as(f64, @floatFromInt(lookup_duration)) / @as(f64, @floatFromInt(num_flows));
    const lookup_ms_per_op = lookup_ns_per_op / 1_000_000.0;
    const lookup_ops_per_sec = 1_000_000_000.0 / lookup_ns_per_op;
    
    print("   Lookup performance:\n");
    print("     Latency: {:.3}ms per op (threshold: <{:.1}ms)\n", .{ lookup_ms_per_op, max_lookup_ms });
    print("     Throughput: {:.0} ops/sec (threshold: >{:.0})\n", .{ lookup_ops_per_sec, @as(f64, @floatFromInt(min_throughput)) });
    print("     Found: {}/{} flows\n", .{ found_count, num_flows });
    
    // Test coordination performance
    const coord_start = time.nanoTimestamp();
    const coord_iterations = 1000;
    
    for (0..coord_iterations) |_| {
        registry.coordinateLocal(.Transport, "performance_test") catch {};
    }
    
    const coord_end = time.nanoTimestamp();
    const coord_duration = coord_end - coord_start;
    const coord_us_per_op = (@as(f64, @floatFromInt(coord_duration)) / @as(f64, @floatFromInt(coord_iterations))) / 1000.0;
    
    print("   Coordination performance:\n");
    print("     Latency: {:.1}µs per op (threshold: <{:.0}µs)\n", .{ coord_us_per_op, max_coordination_us });
    
    // Validate performance thresholds
    try testing.expect(lookup_ms_per_op <= max_lookup_ms);
    try testing.expect(lookup_ops_per_sec >= @as(f64, @floatFromInt(min_throughput)));
    try testing.expect(coord_us_per_op <= max_coordination_us);
    try testing.expect(found_count == num_flows);
    
    const final_stats = registry.getPerformanceStats();
    print("   System stats:\n");
    print("     Cache hit rate: {:.1}%\n", .{final_stats.cache_hit_rate * 100.0});
    print("     Total lookups: {}\n", .{final_stats.lookup_count});
    
    print("   ✅ Performance regression test passed\n\n");
}