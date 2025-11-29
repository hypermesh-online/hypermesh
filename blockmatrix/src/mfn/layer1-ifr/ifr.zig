const std = @import("std");
const print = std.debug.print;
const Allocator = std.mem.Allocator;
const ArrayList = std.ArrayList;
const HashMap = std.HashMap;
const time = std.time;

// Core IFR components
pub const ExactMatcher = @import("exact_matcher.zig").ExactMatcher;
pub const BloomFilterBank = @import("bloom_filter.zig").BloomFilterBank;
pub const UnixSocketServer = @import("unix_socket.zig").UnixSocketServer;
pub const FlowCache = @import("flow_cache.zig").FlowCache;
pub const IFRMetrics = @import("metrics.zig").IFRMetrics;
pub const ComponentIntegration = @import("component_integration.zig").ComponentIntegration;

// Type definitions
pub const FlowRecord = struct {
    key: [32]u8,           // Blake3 hash of flow identifier
    component_id: u32,     // Source HyperMesh component
    flow_type: FlowType,   // Command, Data, Event, Metric
    timestamp: u64,        // Nanosecond timestamp
    size: u32,            // Message size in bytes
    priority: u8,         // 0-7 priority level
    
    pub const FlowType = enum(u8) {
        ComponentCommand,   // Control messages between components
        DataTransfer,      // Large data transfers (container images, etc)
        EventNotification, // State changes, alerts, notifications
        MetricsCollection, // Performance and health metrics
        SecurityEvent,     // Security-related notifications
        HealthCheck,       // Component health and liveness checks
    };
};

pub const ComponentId = enum(u32) {
    Transport = 0,
    Consensus = 1,
    Container = 2,
    Security = 3,
    Orchestration = 4,
    Networking = 5,
    Scheduler = 6,
    
    pub fn toString(self: ComponentId) []const u8 {
        return switch (self) {
            .Transport => "transport",
            .Consensus => "consensus",
            .Container => "container",
            .Security => "security",
            .Orchestration => "orchestration",
            .Networking => "networking",
            .Scheduler => "scheduler",
        };
    }
};

pub const IFRError = error{
    OutOfMemory,
    SocketError,
    LookupFailed,
    InsertionFailed,
    ComponentNotFound,
    MessageTooLarge,
    InvalidMessage,
    ConnectionLost,
    PermissionDenied,
    ConfigurationError,
};

// Main IFR Registry implementation
pub const IFRRegistry = struct {
    allocator: Allocator,
    exact_matcher: *ExactMatcher,
    bloom_filters: *BloomFilterBank,
    unix_socket_server: *UnixSocketServer,
    flow_cache: *FlowCache,
    metrics: *IFRMetrics,
    component_integration: *ComponentIntegration,
    
    const Self = @This();
    
    pub fn init(allocator: Allocator) !*Self {
        const self = try allocator.create(Self);
        errdefer allocator.destroy(self);
        
        // Initialize components with performance optimizations
        const exact_matcher = try ExactMatcher.init(allocator, .{
            .max_entries = 10_000_000,
            .hash_algorithm = .Blake3,
        });
        errdefer exact_matcher.deinit();
        
        const bloom_filters = try BloomFilterBank.init(allocator, .{
            .false_positive_rate = 0.01,
            .expected_entries = 1_000_000,
            .hash_functions = 3,
        });
        errdefer bloom_filters.deinit();
        
        const unix_socket_server = try UnixSocketServer.init(allocator, .{
            .socket_path = "/tmp/hypermesh/",
            .max_connections = 1000,
            .buffer_size = 64 * 1024, // 64KB
            .timeout_ms = 1000,
        });
        errdefer unix_socket_server.deinit();
        
        const flow_cache = try FlowCache.init(allocator, .{
            .max_entries = 10_000_000,
            .max_memory = 100 * 1024 * 1024, // 100MB
            .eviction_strategy = .LRU,
        });
        errdefer flow_cache.deinit();
        
        const metrics = try IFRMetrics.init(allocator);
        errdefer metrics.deinit();
        
        const component_integration = try ComponentIntegration.init(allocator, unix_socket_server);
        errdefer component_integration.deinit();
        
        self.* = Self{
            .allocator = allocator,
            .exact_matcher = exact_matcher,
            .bloom_filters = bloom_filters,
            .unix_socket_server = unix_socket_server,
            .flow_cache = flow_cache,
            .metrics = metrics,
            .component_integration = component_integration,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self) void {
        self.component_integration.deinit();
        self.metrics.deinit();
        self.flow_cache.deinit();
        self.unix_socket_server.deinit();
        self.bloom_filters.deinit();
        self.exact_matcher.deinit();
        self.allocator.destroy(self);
    }
    
    pub fn start(self: *Self) !void {
        print("[IFR] Starting Immediate Flow Registry...\n");
        
        // Start Unix socket server
        try self.unix_socket_server.start();
        print("[IFR] Unix socket server started at {s}\n", .{self.unix_socket_server.config.socket_path});
        
        // Initialize component integration
        try self.component_integration.discoverComponents();
        print("[IFR] Component discovery completed\n");
        
        // Start metrics collection
        try self.metrics.startCollection();
        print("[IFR] Metrics collection started\n");
        
        print("[IFR] Ready for ultra-fast flow coordination\n");
    }
    
    pub fn stop(self: *Self) !void {
        print("[IFR] Shutting down Immediate Flow Registry...\n");
        
        try self.metrics.stopCollection();
        try self.unix_socket_server.stop();
        
        print("[IFR] Shutdown complete\n");
    }
    
    // Ultra-fast exact matching - target <0.1ms
    pub fn lookup(self: *Self, key: []const u8) ?FlowRecord {
        const lookup_start = time.nanoTimestamp();
        defer {
            const duration = time.nanoTimestamp() - lookup_start;
            self.metrics.recordLookupLatency(@as(f64, @floatFromInt(duration)) / 1_000_000.0);
        }
        
        // Check bloom filter first for fast negative lookups
        if (!self.bloom_filters.contains(key)) {
            self.metrics.incrementBloomFilterRejects();
            return null;
        }
        
        // Check cache first
        if (self.flow_cache.get(key)) |flow| {
            self.metrics.incrementCacheHits();
            return flow;
        }
        
        // Exact matcher lookup
        const result = self.exact_matcher.find(key);
        if (result) |flow| {
            // Cache the result
            self.flow_cache.put(key, flow) catch |err| {
                print("[IFR] Warning: Failed to cache flow record: {}\n", .{err});
            };
            self.metrics.incrementCacheMisses();
        }
        
        return result;
    }
    
    // Flow registration and management
    pub fn registerFlow(self: *Self, flow: FlowRecord) !void {
        const reg_start = time.nanoTimestamp();
        defer {
            const duration = time.nanoTimestamp() - reg_start;
            self.metrics.recordRegistrationLatency(@as(f64, @floatFromInt(duration)) / 1_000_000.0);
        }
        
        // Insert into exact matcher
        try self.exact_matcher.insert(&flow.key, flow);
        
        // Add to bloom filter
        self.bloom_filters.add(&flow.key);
        
        // Cache the flow
        try self.flow_cache.put(&flow.key, flow);
        
        // Update metrics
        self.metrics.incrementFlowRegistrations();
        
        print("[IFR] Registered flow: component_id={}, type={}, size={}\n", .{
            flow.component_id,
            @intFromEnum(flow.flow_type),
            flow.size,
        });
    }
    
    // Unix socket IPC coordination
    pub fn coordinateLocal(self: *Self, component: ComponentId, message: []const u8) !void {
        const coord_start = time.nanoTimestamp();
        defer {
            const duration = time.nanoTimestamp() - coord_start;
            self.metrics.recordCoordinationLatency(@as(f64, @floatFromInt(duration)) / 1_000_000.0);
        }
        
        try self.component_integration.sendToComponent(component, message);
        self.metrics.incrementCoordinationMessages();
    }
    
    // Performance status
    pub fn getPerformanceStats(self: *Self) IFRMetrics.PerformanceStats {
        return self.metrics.getStats();
    }
    
    // Health check for component monitoring
    pub fn healthCheck(self: *Self) bool {
        return self.unix_socket_server.isHealthy() and
               self.exact_matcher.isHealthy() and
               self.flow_cache.isHealthy();
    }
};

// C FFI exports for Rust integration
export fn ifr_create() ?*IFRRegistry {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();
    
    return IFRRegistry.init(allocator) catch null;
}

export fn ifr_destroy(registry: ?*IFRRegistry) void {
    if (registry) |reg| {
        reg.deinit();
    }
}

export fn ifr_start(registry: ?*IFRRegistry) bool {
    if (registry) |reg| {
        reg.start() catch return false;
        return true;
    }
    return false;
}

export fn ifr_stop(registry: ?*IFRRegistry) bool {
    if (registry) |reg| {
        reg.stop() catch return false;
        return true;
    }
    return false;
}

export fn ifr_lookup(registry: ?*IFRRegistry, key: [*c]const u8, key_len: usize) bool {
    if (registry) |reg| {
        const key_slice = key[0..key_len];
        return reg.lookup(key_slice) != null;
    }
    return false;
}

export fn ifr_register_flow(registry: ?*IFRRegistry, 
                           key: [*c]const u8, key_len: usize,
                           component_id: u32, flow_type: u8, 
                           size: u32, priority: u8) bool {
    if (registry) |reg| {
        const key_slice = key[0..key_len];
        var flow_key: [32]u8 = undefined;
        if (key_len <= 32) {
            @memcpy(flow_key[0..key_len], key_slice);
            if (key_len < 32) {
                @memset(flow_key[key_len..], 0);
            }
        } else {
            return false;
        }
        
        const flow = FlowRecord{
            .key = flow_key,
            .component_id = component_id,
            .flow_type = @enumFromInt(flow_type),
            .timestamp = @intCast(time.nanoTimestamp()),
            .size = size,
            .priority = priority,
        };
        
        reg.registerFlow(flow) catch return false;
        return true;
    }
    return false;
}

export fn ifr_coordinate_local(registry: ?*IFRRegistry, 
                              component_id: u32, 
                              message: [*c]const u8, message_len: usize) bool {
    if (registry) |reg| {
        const message_slice = message[0..message_len];
        const component: ComponentId = @enumFromInt(component_id);
        reg.coordinateLocal(component, message_slice) catch return false;
        return true;
    }
    return false;
}

export fn ifr_health_check(registry: ?*IFRRegistry) bool {
    if (registry) |reg| {
        return reg.healthCheck();
    }
    return false;
}

// Test support
test "IFR basic functionality" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const registry = try IFRRegistry.init(allocator);
    defer registry.deinit();
    
    // Test flow registration and lookup
    const test_key = "test_flow_key";
    const test_flow = FlowRecord{
        .key = std.mem.zeroes([32]u8),
        .component_id = @intFromEnum(ComponentId.Transport),
        .flow_type = .ComponentCommand,
        .timestamp = @intCast(time.nanoTimestamp()),
        .size = 1024,
        .priority = 5,
    };
    
    @memcpy(test_flow.key[0..test_key.len], test_key);
    
    try registry.registerFlow(test_flow);
    
    const result = registry.lookup(test_key);
    try testing.expect(result != null);
    try testing.expect(result.?.component_id == @intFromEnum(ComponentId.Transport));
    try testing.expect(result.?.flow_type == .ComponentCommand);
}