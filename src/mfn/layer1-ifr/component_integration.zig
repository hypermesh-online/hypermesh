const std = @import("std");
const print = std.debug.print;
const Allocator = std.mem.Allocator;
const HashMap = std.HashMap;
const ArrayList = std.ArrayList;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const time = std.time;
const fs = std.fs;

const ComponentId = @import("ifr.zig").ComponentId;
const IFRError = @import("ifr.zig").IFRError;
const UnixSocketServer = @import("unix_socket.zig").UnixSocketServer;
const MessageType = @import("unix_socket.zig").MessageType;

// Component information and status
pub const ComponentInfo = struct {
    id: ComponentId,
    socket_path: []const u8,
    pid: ?u32,
    status: ComponentStatus,
    last_heartbeat: u64,
    version: []const u8,
    capabilities: []const []const u8,
    
    const ComponentStatus = enum {
        Unknown,
        Starting,
        Running,
        Stopping,
        Stopped,
        Failed,
    };
    
    pub fn init(allocator: Allocator, id: ComponentId, socket_path: []const u8) !ComponentInfo {
        return ComponentInfo{
            .id = id,
            .socket_path = try allocator.dupe(u8, socket_path),
            .pid = null,
            .status = .Unknown,
            .last_heartbeat = 0,
            .version = try allocator.dupe(u8, "unknown"),
            .capabilities = &[_][]const u8{},
        };
    }
    
    pub fn deinit(self: *ComponentInfo, allocator: Allocator) void {
        allocator.free(self.socket_path);
        allocator.free(self.version);
        for (self.capabilities) |capability| {
            allocator.free(capability);
        }
        allocator.free(self.capabilities);
    }
    
    pub fn isHealthy(self: *const ComponentInfo, timeout_seconds: u64) bool {
        const now = @as(u64, @intCast(time.timestamp()));
        return self.status == .Running and
               (self.last_heartbeat > 0) and
               (now - self.last_heartbeat < timeout_seconds);
    }
};

// Component discovery and integration manager
pub const ComponentIntegration = struct {
    allocator: Allocator,
    unix_socket_server: *UnixSocketServer,
    
    // Component registry
    components: HashMap(ComponentId, ComponentInfo, ComponentHashContext, std.hash_map.default_max_load_percentage),
    components_mutex: Mutex,
    
    // Discovery settings
    socket_base_path: []const u8,
    discovery_interval: u64, // seconds
    heartbeat_timeout: u64,  // seconds
    
    // Discovery thread
    discovery_thread: ?Thread,
    discovery_running: bool,
    
    const Self = @This();
    
    const ComponentHashContext = struct {
        pub fn hash(self: @This(), key: ComponentId) u64 {
            _ = self;
            return @intFromEnum(key);
        }
        
        pub fn eql(self: @This(), a: ComponentId, b: ComponentId) bool {
            _ = self;
            return a == b;
        }
    };
    
    pub fn init(allocator: Allocator, unix_socket_server: *UnixSocketServer) !*Self {
        const self = try allocator.create(Self);
        errdefer allocator.destroy(self);
        
        const components = HashMap(ComponentId, ComponentInfo, ComponentHashContext, std.hash_map.default_max_load_percentage).init(allocator);
        
        self.* = Self{
            .allocator = allocator,
            .unix_socket_server = unix_socket_server,
            .components = components,
            .components_mutex = Mutex{},
            .socket_base_path = "/tmp/hypermesh/",
            .discovery_interval = 30, // 30 seconds
            .heartbeat_timeout = 60,  // 60 seconds
            .discovery_thread = null,
            .discovery_running = false,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self) void {
        if (self.discovery_running) {
            self.stopDiscovery() catch {};
        }
        
        self.components_mutex.lock();
        var iterator = self.components.iterator();
        while (iterator.next()) |entry| {
            var component_info = entry.value_ptr;
            component_info.deinit(self.allocator);
        }
        self.components.deinit();
        self.components_mutex.unlock();
        
        self.allocator.destroy(self);
    }
    
    // Discover HyperMesh components
    pub fn discoverComponents(self: *Self) !void {
        print("[ComponentIntegration] Starting component discovery...\n");
        
        // Start discovery thread
        if (!self.discovery_running) {
            self.discovery_running = true;
            self.discovery_thread = try Thread.spawn(.{}, discoveryThread, .{self});
        }
        
        // Perform initial discovery
        try self.scanComponents();
        
        print("[ComponentIntegration] Component discovery initialized\n");
    }
    
    pub fn stopDiscovery(self: *Self) !void {
        if (!self.discovery_running) return;
        
        self.discovery_running = false;
        
        if (self.discovery_thread) |thread| {
            thread.join();
            self.discovery_thread = null;
        }
        
        print("[ComponentIntegration] Component discovery stopped\n");
    }
    
    // Send message to specific component
    pub fn sendToComponent(self: *Self, component: ComponentId, message: []const u8) !void {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        if (self.components.get(component)) |component_info| {
            if (!component_info.isHealthy(self.heartbeat_timeout)) {
                return IFRError.ComponentNotFound;
            }
            
            // Use Unix socket server to send message
            try self.unix_socket_server.send(component, message);
            
            print("[ComponentIntegration] Sent message to {s}: {} bytes\n", .{ component.toString(), message.len });
        } else {
            return IFRError.ComponentNotFound;
        }
    }
    
    // Broadcast message to all healthy components
    pub fn broadcastToAll(self: *Self, message: []const u8) !void {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        var sent_count: usize = 0;
        var iterator = self.components.iterator();
        
        while (iterator.next()) |entry| {
            const component_info = entry.value_ptr;
            
            if (component_info.isHealthy(self.heartbeat_timeout)) {
                self.unix_socket_server.send(entry.key_ptr.*, message) catch |err| {
                    print("[ComponentIntegration] Failed to send to {s}: {}\n", .{ entry.key_ptr.toString(), err });
                    continue;
                };
                sent_count += 1;
            }
        }
        
        print("[ComponentIntegration] Broadcast message to {} components: {} bytes\n", .{ sent_count, message.len });
    }
    
    // Get component status
    pub fn getComponentStatus(self: *Self, component: ComponentId) ?ComponentInfo {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        return self.components.get(component);
    }
    
    // Get all component statuses
    pub fn getAllComponents(self: *Self) ![]ComponentInfo {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        var result = try self.allocator.alloc(ComponentInfo, self.components.count());
        var i: usize = 0;
        
        var iterator = self.components.iterator();
        while (iterator.next()) |entry| {
            result[i] = entry.value_ptr.*;
            i += 1;
        }
        
        return result;
    }
    
    // Get discovery statistics
    pub fn getStats(self: *Self) Stats {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        var running_count: usize = 0;
        var healthy_count: usize = 0;
        var failed_count: usize = 0;
        
        var iterator = self.components.iterator();
        while (iterator.next()) |entry| {
            const component_info = entry.value_ptr;
            
            switch (component_info.status) {
                .Running => running_count += 1,
                .Failed => failed_count += 1,
                else => {},
            }
            
            if (component_info.isHealthy(self.heartbeat_timeout)) {
                healthy_count += 1;
            }
        }
        
        return Stats{
            .total_components = self.components.count(),
            .running_components = running_count,
            .healthy_components = healthy_count,
            .failed_components = failed_count,
            .discovery_running = self.discovery_running,
        };
    }
    
    pub const Stats = struct {
        total_components: usize,
        running_components: usize,
        healthy_components: usize,
        failed_components: usize,
        discovery_running: bool,
    };
    
    // Register component manually
    pub fn registerComponent(self: *Self, component: ComponentId, socket_path: []const u8, pid: ?u32) !void {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        var component_info = try ComponentInfo.init(self.allocator, component, socket_path);
        component_info.pid = pid;
        component_info.status = .Running;
        component_info.last_heartbeat = @intCast(time.timestamp());
        
        try self.components.put(component, component_info);
        
        print("[ComponentIntegration] Registered component {s} at {s}\n", .{ component.toString(), socket_path });
    }
    
    // Update component heartbeat
    pub fn updateHeartbeat(self: *Self, component: ComponentId) void {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        if (self.components.getPtr(component)) |component_info| {
            component_info.last_heartbeat = @intCast(time.timestamp());
            component_info.status = .Running;
        }
    }
    
    // Mark component as failed
    pub fn markComponentFailed(self: *Self, component: ComponentId) void {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        if (self.components.getPtr(component)) |component_info| {
            component_info.status = .Failed;
        }
    }
    
    // Print component status report
    pub fn printStatusReport(self: *Self) void {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        const stats = self.getStats();
        
        print("\n=== Component Integration Status ===\n");
        print("Total components: {}\n", .{stats.total_components});
        print("Running components: {}\n", .{stats.running_components});
        print("Healthy components: {}\n", .{stats.healthy_components});
        print("Failed components: {}\n", .{stats.failed_components});
        print("Discovery running: {}\n", .{stats.discovery_running});
        
        print("\nComponent Details:\n");
        var iterator = self.components.iterator();
        while (iterator.next()) |entry| {
            const component_info = entry.value_ptr;
            const healthy_status = if (component_info.isHealthy(self.heartbeat_timeout)) "✅" else "❌";
            const last_heartbeat_ago = @as(u64, @intCast(time.timestamp())) - component_info.last_heartbeat;
            
            print("  {s} {s}: {} (heartbeat: {}s ago, pid: {})\n", .{
                healthy_status,
                entry.key_ptr.toString(),
                component_info.status,
                last_heartbeat_ago,
                component_info.pid orelse 0,
            });
        }
        print("=====================================\n\n");
    }
    
    // Private methods
    
    fn scanComponents(self: *Self) !void {
        // Scan socket directory for component sockets
        const socket_dir = fs.openDirAbsolute(self.socket_base_path, .{ .iterate = true }) catch |err| {
            print("[ComponentIntegration] Cannot open socket directory {s}: {}\n", .{ self.socket_base_path, err });
            return;
        };
        defer socket_dir.close();
        
        var iterator = socket_dir.iterate();
        while (try iterator.next()) |entry| {
            if (entry.kind == .unix_domain_socket) {
                try self.processSocketFile(entry.name);
            }
        }
    }
    
    fn processSocketFile(self: *Self, filename: []const u8) !void {
        // Parse component from socket filename (e.g., "transport.sock" -> Transport)
        if (std.mem.endsWith(u8, filename, ".sock")) {
            const component_name = filename[0 .. filename.len - 5]; // Remove ".sock"
            
            const component_id = self.parseComponentName(component_name);
            if (component_id == null) {
                print("[ComponentIntegration] Unknown component: {s}\n", .{component_name});
                return;
            }
            
            const socket_path = try std.fmt.allocPrint(self.allocator, "{s}{s}", .{ self.socket_base_path, filename });
            defer self.allocator.free(socket_path);
            
            // Check if component is already registered
            self.components_mutex.lock();
            const exists = self.components.contains(component_id.?);
            self.components_mutex.unlock();
            
            if (!exists) {
                // Try to get PID from process info if possible
                const pid = self.getComponentPid(component_name);
                try self.registerComponent(component_id.?, socket_path, pid);
            }
        }
    }
    
    fn parseComponentName(self: *Self, name: []const u8) ?ComponentId {
        _ = self;
        
        if (std.mem.eql(u8, name, "transport")) return .Transport;
        if (std.mem.eql(u8, name, "consensus")) return .Consensus;
        if (std.mem.eql(u8, name, "container")) return .Container;
        if (std.mem.eql(u8, name, "security")) return .Security;
        if (std.mem.eql(u8, name, "orchestration")) return .Orchestration;
        if (std.mem.eql(u8, name, "networking")) return .Networking;
        if (std.mem.eql(u8, name, "scheduler")) return .Scheduler;
        
        return null;
    }
    
    fn getComponentPid(self: *Self, component_name: []const u8) ?u32 {
        _ = self;
        _ = component_name;
        
        // TODO: Implement PID detection via process scanning
        // For now, return null - PID detection is optional
        return null;
    }
    
    fn discoveryThread(self: *Self) void {
        print("[ComponentIntegration] Discovery thread started\n");
        
        while (self.discovery_running) {
            // Scan for new components
            self.scanComponents() catch |err| {
                print("[ComponentIntegration] Discovery scan failed: {}\n", .{err});
            };
            
            // Check component health and clean up failed components
            self.healthCheck();
            
            // Wait for next discovery interval
            const sleep_ms = self.discovery_interval * 1000;
            for (0..sleep_ms) |_| {
                if (!self.discovery_running) break;
                std.time.sleep(1_000_000); // 1ms
            }
        }
        
        print("[ComponentIntegration] Discovery thread stopped\n");
    }
    
    fn healthCheck(self: *Self) void {
        self.components_mutex.lock();
        defer self.components_mutex.unlock();
        
        var to_remove = ArrayList(ComponentId).init(self.allocator);
        defer to_remove.deinit();
        
        var iterator = self.components.iterator();
        while (iterator.next()) |entry| {
            const component_info = entry.value_ptr;
            
            if (!component_info.isHealthy(self.heartbeat_timeout)) {
                if (component_info.status != .Failed) {
                    print("[ComponentIntegration] Component {s} unhealthy, marking as failed\n", .{entry.key_ptr.toString()});
                    component_info.status = .Failed;
                }
                
                // Remove components that have been failed for too long
                const now = @as(u64, @intCast(time.timestamp()));
                if (component_info.last_heartbeat > 0 and (now - component_info.last_heartbeat) > (self.heartbeat_timeout * 3)) {
                    to_remove.append(entry.key_ptr.*) catch continue;
                }
            }
        }
        
        // Remove expired components
        for (to_remove.items) |component_id| {
            if (self.components.fetchRemove(component_id)) |entry| {
                var component_info = entry.value;
                component_info.deinit(self.allocator);
                print("[ComponentIntegration] Removed expired component {s}\n", .{component_id.toString()});
            }
        }
    }
};

// Mock components for testing
pub const MockComponent = struct {
    component_id: ComponentId,
    socket_path: []const u8,
    running: bool,
    
    const Self = @This();
    
    pub fn init(component_id: ComponentId, socket_path: []const u8) Self {
        return Self{
            .component_id = component_id,
            .socket_path = socket_path,
            .running = false,
        };
    }
    
    pub fn start(self: *Self) !void {
        // Create mock socket file
        const file = try fs.createFileAbsolute(self.socket_path, .{});
        file.close();
        
        self.running = true;
        print("[MockComponent] Started {s} at {s}\n", .{ self.component_id.toString(), self.socket_path });
    }
    
    pub fn stop(self: *Self) void {
        if (self.running) {
            fs.deleteFileAbsolute(self.socket_path) catch {};
            self.running = false;
            print("[MockComponent] Stopped {s}\n", .{self.component_id.toString()});
        }
    }
};

// Integration tests
pub fn testComponentIntegration(allocator: Allocator) !void {
    print("[ComponentIntegration] Running integration test...\n");
    
    // Create test socket directory
    const test_dir = "/tmp/hypermesh_test/";
    fs.makeDirAbsolute(test_dir) catch {};
    defer {
        // Cleanup test directory
        if (fs.openDirAbsolute(test_dir, .{})) |dir| {
            dir.deleteTree(".") catch {};
            dir.close();
        } else |_| {}
    }
    
    // Create mock Unix socket server
    const mock_server = try UnixSocketServer.init(allocator, .{
        .socket_path = test_dir,
    });
    defer mock_server.deinit();
    
    // Create component integration
    const integration = try ComponentIntegration.init(allocator, mock_server);
    defer integration.deinit();
    
    // Create mock components
    const transport_path = try std.fmt.allocPrint(allocator, "{s}transport.sock", .{test_dir});
    defer allocator.free(transport_path);
    
    const consensus_path = try std.fmt.allocPrint(allocator, "{s}consensus.sock", .{test_dir});
    defer allocator.free(consensus_path);
    
    var transport_mock = MockComponent.init(.Transport, transport_path);
    var consensus_mock = MockComponent.init(.Consensus, consensus_path);
    
    try transport_mock.start();
    defer transport_mock.stop();
    
    try consensus_mock.start();
    defer consensus_mock.stop();
    
    // Test discovery
    try integration.discoverComponents();
    
    // Allow discovery to run
    std.time.sleep(100_000_000); // 100ms
    
    // Check discovered components
    const stats = integration.getStats();
    print("[ComponentIntegration] Discovered {} components\n", .{stats.total_components});
    
    // Test component communication
    const test_message = "Hello from IFR!";
    integration.sendToComponent(.Transport, test_message) catch |err| {
        print("[ComponentIntegration] Send test (expected to fail): {}\n", .{err});
    };
    
    // Print status report
    integration.printStatusReport();
    
    try integration.stopDiscovery();
    
    print("[ComponentIntegration] Integration test completed\n");
}

// Unit tests
test "ComponentInfo basic operations" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const component_info = try ComponentInfo.init(allocator, .Transport, "/tmp/test.sock");
    defer {
        var mutable_info = component_info;
        mutable_info.deinit(allocator);
    }
    
    try testing.expect(component_info.id == .Transport);
    try testing.expect(std.mem.eql(u8, component_info.socket_path, "/tmp/test.sock"));
    try testing.expect(component_info.status == .Unknown);
}

test "ComponentIntegration creation" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const mock_server = try UnixSocketServer.init(allocator, .{});
    defer mock_server.deinit();
    
    const integration = try ComponentIntegration.init(allocator, mock_server);
    defer integration.deinit();
    
    const stats = integration.getStats();
    try testing.expect(stats.total_components == 0);
    try testing.expect(!stats.discovery_running);
}

test "Component integration" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    try testComponentIntegration(allocator);
}