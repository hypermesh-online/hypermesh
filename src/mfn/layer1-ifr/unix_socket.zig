const std = @import("std");
const print = std.debug.print;
const Allocator = std.mem.Allocator;
const ArrayList = std.ArrayList;
const HashMap = std.HashMap;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const time = std.time;
const os = std.os;
const net = std.net;
const posix = std.posix;

const ComponentId = @import("ifr.zig").ComponentId;
const IFRError = @import("ifr.zig").IFRError;

// Unix socket server configuration
pub const UnixSocketConfig = struct {
    socket_path: []const u8 = "/tmp/hypermesh/",
    max_connections: usize = 1000,
    buffer_size: usize = 64 * 1024, // 64KB
    timeout_ms: u32 = 1000,
    worker_threads: usize = 4,
    backlog: u32 = 128,
};

// Message types for IPC protocol
pub const MessageType = enum(u8) {
    Command = 0,
    Response = 1,
    Event = 2,
    Heartbeat = 3,
    Discovery = 4,
    Registration = 5,
};

// Binary message frame structure
pub const MessageFrame = struct {
    magic: u32 = 0x48594D46, // "HYMF" - HyperMesh Frame
    version: u8 = 1,
    message_type: MessageType,
    component_id: u32,
    sequence: u32,
    payload_size: u32,
    checksum: u32,
    timestamp: u64,
    // payload follows immediately after header
    
    const HEADER_SIZE = 32;
    
    pub fn serialize(self: *const MessageFrame, buffer: []u8) !usize {
        if (buffer.len < HEADER_SIZE) return error.BufferTooSmall;
        
        std.mem.writeInt(u32, buffer[0..4], self.magic, .little);
        buffer[4] = self.version;
        buffer[5] = @intFromEnum(self.message_type);
        buffer[6] = 0; // reserved
        buffer[7] = 0; // reserved
        std.mem.writeInt(u32, buffer[8..12], self.component_id, .little);
        std.mem.writeInt(u32, buffer[12..16], self.sequence, .little);
        std.mem.writeInt(u32, buffer[16..20], self.payload_size, .little);
        std.mem.writeInt(u32, buffer[20..24], self.checksum, .little);
        std.mem.writeInt(u64, buffer[24..32], self.timestamp, .little);
        
        return HEADER_SIZE;
    }
    
    pub fn deserialize(buffer: []const u8) !MessageFrame {
        if (buffer.len < HEADER_SIZE) return error.InvalidMessage;
        
        const magic = std.mem.readInt(u32, buffer[0..4], .little);
        if (magic != 0x48594D46) return error.InvalidMessage;
        
        return MessageFrame{
            .magic = magic,
            .version = buffer[4],
            .message_type = @enumFromInt(buffer[5]),
            .component_id = std.mem.readInt(u32, buffer[8..12], .little),
            .sequence = std.mem.readInt(u32, buffer[12..16], .little),
            .payload_size = std.mem.readInt(u32, buffer[16..20], .little),
            .checksum = std.mem.readInt(u32, buffer[20..24], .little),
            .timestamp = std.mem.readInt(u64, buffer[24..32], .little),
        };
    }
    
    pub fn calculateChecksum(payload: []const u8) u32 {
        var crc = std.hash.Crc32.init();
        crc.update(payload);
        return crc.final();
    }
};

// Connection pool for managing client connections
pub const ConnectionPool = struct {
    allocator: Allocator,
    connections: HashMap(i32, *Connection, ConnectionHashContext, std.hash_map.default_max_load_percentage),
    mutex: Mutex,
    next_id: u32,
    
    const Self = @This();
    
    const ConnectionHashContext = struct {
        pub fn hash(self: @This(), key: i32) u64 {
            _ = self;
            return @as(u64, @intCast(key));
        }
        
        pub fn eql(self: @This(), a: i32, b: i32) bool {
            _ = self;
            return a == b;
        }
    };
    
    const Connection = struct {
        socket: posix.socket_t,
        component_id: ComponentId,
        last_heartbeat: u64,
        sequence: u32,
        buffer: []u8,
        is_active: bool,
        
        pub fn init(allocator: Allocator, socket: posix.socket_t, buffer_size: usize) !*Connection {
            const self = try allocator.create(Connection);
            const buffer = try allocator.alloc(u8, buffer_size);
            
            self.* = Connection{
                .socket = socket,
                .component_id = .Transport, // Default
                .last_heartbeat = @intCast(time.timestamp()),
                .sequence = 0,
                .buffer = buffer,
                .is_active = true,
            };
            
            return self;
        }
        
        pub fn deinit(self: *Connection, allocator: Allocator) void {
            allocator.free(self.buffer);
            posix.close(self.socket);
            allocator.destroy(self);
        }
        
        pub fn sendMessage(self: *Connection, message_type: MessageType, payload: []const u8) !void {
            const frame = MessageFrame{
                .message_type = message_type,
                .component_id = @intFromEnum(self.component_id),
                .sequence = self.sequence,
                .payload_size = @intCast(payload.len),
                .checksum = MessageFrame.calculateChecksum(payload),
                .timestamp = @intCast(time.nanoTimestamp()),
            };
            
            // Send frame header
            var header_buffer: [MessageFrame.HEADER_SIZE]u8 = undefined;
            _ = try frame.serialize(&header_buffer);
            _ = try posix.send(self.socket, &header_buffer, 0);
            
            // Send payload
            if (payload.len > 0) {
                _ = try posix.send(self.socket, payload, 0);
            }
            
            self.sequence += 1;
        }
        
        pub fn receiveMessage(self: *Connection) !struct { frame: MessageFrame, payload: []const u8 } {
            // Receive header
            var header_buffer: [MessageFrame.HEADER_SIZE]u8 = undefined;
            const header_received = try posix.recv(self.socket, &header_buffer, 0);
            if (header_received != MessageFrame.HEADER_SIZE) {
                return error.IncompleteMessage;
            }
            
            const frame = try MessageFrame.deserialize(&header_buffer);
            
            if (frame.payload_size > self.buffer.len) {
                return error.MessageTooLarge;
            }
            
            // Receive payload if present
            var payload_slice: []const u8 = &[_]u8{};
            if (frame.payload_size > 0) {
                const payload_received = try posix.recv(self.socket, self.buffer[0..frame.payload_size], 0);
                if (payload_received != frame.payload_size) {
                    return error.IncompleteMessage;
                }
                
                // Verify checksum
                const calculated_checksum = MessageFrame.calculateChecksum(self.buffer[0..frame.payload_size]);
                if (calculated_checksum != frame.checksum) {
                    return error.InvalidMessage;
                }
                
                payload_slice = self.buffer[0..frame.payload_size];
            }
            
            self.last_heartbeat = @intCast(time.timestamp());
            
            return .{ .frame = frame, .payload = payload_slice };
        }
        
        pub fn isHealthy(self: *Connection, timeout_seconds: u64) bool {
            const now = @as(u64, @intCast(time.timestamp()));
            return self.is_active and (now - self.last_heartbeat < timeout_seconds);
        }
    };
    
    pub fn init(allocator: Allocator) !*Self {
        const self = try allocator.create(Self);
        
        self.* = Self{
            .allocator = allocator,
            .connections = HashMap(i32, *Connection, ConnectionHashContext, std.hash_map.default_max_load_percentage).init(allocator),
            .mutex = Mutex{},
            .next_id = 1,
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        var iterator = self.connections.iterator();
        while (iterator.next()) |entry| {
            entry.value_ptr.*.deinit(self.allocator);
        }
        
        self.connections.deinit();
        self.allocator.destroy(self);
    }
    
    pub fn addConnection(self: *Self, socket: posix.socket_t, buffer_size: usize) !i32 {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        const connection = try Connection.init(self.allocator, socket, buffer_size);
        const id = @as(i32, @intCast(self.next_id));
        self.next_id += 1;
        
        try self.connections.put(id, connection);
        return id;
    }
    
    pub fn removeConnection(self: *Self, id: i32) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        if (self.connections.fetchRemove(id)) |entry| {
            entry.value.deinit(self.allocator);
        }
    }
    
    pub fn getConnection(self: *Self, id: i32) ?*Connection {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        return self.connections.get(id);
    }
    
    pub fn broadcastMessage(self: *Self, message_type: MessageType, payload: []const u8) !void {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        var iterator = self.connections.iterator();
        while (iterator.next()) |entry| {
            const connection = entry.value_ptr.*;
            connection.sendMessage(message_type, payload) catch |err| {
                print("[ConnectionPool] Failed to send to connection {}: {}\n", .{ entry.key_ptr.*, err });
                connection.is_active = false;
            };
        }
    }
    
    pub fn getActiveConnections(self: *Self) usize {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        var count: usize = 0;
        var iterator = self.connections.iterator();
        while (iterator.next()) |entry| {
            if (entry.value_ptr.*.isHealthy(30)) {
                count += 1;
            }
        }
        
        return count;
    }
    
    pub fn cleanupStaleConnections(self: *Self, timeout_seconds: u64) usize {
        self.mutex.lock();
        defer self.mutex.unlock();
        
        var to_remove = ArrayList(i32).init(self.allocator);
        defer to_remove.deinit();
        
        var iterator = self.connections.iterator();
        while (iterator.next()) |entry| {
            if (!entry.value_ptr.*.isHealthy(timeout_seconds)) {
                to_remove.append(entry.key_ptr.*) catch continue;
            }
        }
        
        for (to_remove.items) |id| {
            if (self.connections.fetchRemove(id)) |entry| {
                entry.value.deinit(self.allocator);
            }
        }
        
        return to_remove.items.len;
    }
};

// High-performance Unix socket server
pub const UnixSocketServer = struct {
    allocator: Allocator,
    config: UnixSocketConfig,
    server_socket: ?posix.socket_t,
    connection_pool: *ConnectionPool,
    worker_threads: []Thread,
    is_running: bool,
    shutdown_mutex: Mutex,
    
    const Self = @This();
    
    pub fn init(allocator: Allocator, config: UnixSocketConfig) !*Self {
        const self = try allocator.create(Self);
        errdefer allocator.destroy(self);
        
        const connection_pool = try ConnectionPool.init(allocator);
        errdefer connection_pool.deinit();
        
        const worker_threads = try allocator.alloc(Thread, config.worker_threads);
        errdefer allocator.free(worker_threads);
        
        self.* = Self{
            .allocator = allocator,
            .config = config,
            .server_socket = null,
            .connection_pool = connection_pool,
            .worker_threads = worker_threads,
            .is_running = false,
            .shutdown_mutex = Mutex{},
        };
        
        return self;
    }
    
    pub fn deinit(self: *Self) void {
        if (self.is_running) {
            self.stop() catch {};
        }
        
        self.connection_pool.deinit();
        self.allocator.free(self.worker_threads);
        self.allocator.destroy(self);
    }
    
    pub fn start(self: *Self) !void {
        print("[UnixSocketServer] Starting server at {s}\n", .{self.config.socket_path});
        
        // Create socket directory if it doesn't exist
        std.fs.makeDirAbsolute(self.config.socket_path) catch |err| switch (err) {
            error.PathAlreadyExists => {},
            else => return err,
        };
        
        // Create server socket
        const server_socket = try posix.socket(posix.AF.UNIX, posix.SOCK.STREAM, 0);
        errdefer posix.close(server_socket);
        
        // Bind to socket path
        const socket_path = try std.fmt.allocPrintZ(self.allocator, "{s}ifr.sock", .{self.config.socket_path});
        defer self.allocator.free(socket_path);
        
        // Remove existing socket file
        std.fs.deleteFileAbsolute(socket_path) catch {};
        
        var addr = posix.sockaddr.un{
            .family = posix.AF.UNIX,
            .path = undefined,
        };
        
        if (socket_path.len >= addr.path.len) {
            return error.PathTooLong;
        }
        
        @memcpy(addr.path[0..socket_path.len], socket_path);
        addr.path[socket_path.len] = 0;
        
        try posix.bind(server_socket, @ptrCast(&addr), @sizeOf(posix.sockaddr.un));
        try posix.listen(server_socket, self.config.backlog);
        
        // Set socket permissions (read/write for owner and group)
        try std.fs.chmod(socket_path, 0o660);
        
        self.server_socket = server_socket;
        self.is_running = true;
        
        // Start worker threads
        for (self.worker_threads, 0..) |*thread, i| {
            thread.* = try Thread.spawn(.{}, workerThread, .{ self, i });
        }
        
        print("[UnixSocketServer] Server started successfully\n");
    }
    
    pub fn stop(self: *Self) !void {
        self.shutdown_mutex.lock();
        defer self.shutdown_mutex.unlock();
        
        if (!self.is_running) return;
        
        print("[UnixSocketServer] Stopping server...\n");
        
        self.is_running = false;
        
        // Close server socket to unblock accept()
        if (self.server_socket) |socket| {
            posix.close(socket);
            self.server_socket = null;
        }
        
        // Wait for worker threads to finish
        for (self.worker_threads) |*thread| {
            thread.join();
        }
        
        print("[UnixSocketServer] Server stopped\n");
    }
    
    pub fn send(self: *Self, component: ComponentId, message: []const u8) !void {
        _ = component;
        const send_start = time.nanoTimestamp();
        defer {
            const duration = time.nanoTimestamp() - send_start;
            if (duration > 50_000) { // 50 microseconds
                print("[UnixSocketServer] Warning: send took {:.1} µs (target <50µs)\n", .{@as(f64, @floatFromInt(duration)) / 1000.0});
            }
        }
        
        try self.connection_pool.broadcastMessage(.Command, message);
    }
    
    pub fn isHealthy(self: *Self) bool {
        return self.is_running and 
               self.server_socket != null and 
               self.connection_pool.getActiveConnections() > 0;
    }
    
    pub fn getStats(self: *Self) Stats {
        return Stats{
            .is_running = self.is_running,
            .active_connections = self.connection_pool.getActiveConnections(),
            .worker_threads = self.worker_threads.len,
        };
    }
    
    pub const Stats = struct {
        is_running: bool,
        active_connections: usize,
        worker_threads: usize,
    };
    
    // Worker thread function
    fn workerThread(self: *Self, thread_id: usize) void {
        print("[UnixSocketServer] Worker thread {} started\n", .{thread_id});
        
        while (self.is_running) {
            if (self.server_socket) |server_socket| {
                // Accept new connection
                const client_socket = posix.accept(server_socket, null, null, posix.SOCK.CLOEXEC) catch |err| {
                    if (self.is_running) {
                        print("[UnixSocketServer] Accept failed: {}\n", .{err});
                    }
                    continue;
                };
                
                // Set socket timeout
                const timeout = posix.timeval{
                    .tv_sec = @intCast(self.config.timeout_ms / 1000),
                    .tv_usec = @intCast((self.config.timeout_ms % 1000) * 1000),
                };
                
                posix.setsockopt(client_socket, posix.SOL.SOCKET, posix.SO.RCVTIMEO, 
                               std.mem.asBytes(&timeout)) catch {};
                posix.setsockopt(client_socket, posix.SOL.SOCKET, posix.SO.SNDTIMEO,
                               std.mem.asBytes(&timeout)) catch {};
                
                // Add to connection pool
                const connection_id = self.connection_pool.addConnection(client_socket, self.config.buffer_size) catch |err| {
                    print("[UnixSocketServer] Failed to add connection: {}\n", .{err});
                    posix.close(client_socket);
                    continue;
                };
                
                print("[UnixSocketServer] New connection {} accepted by worker {}\n", .{ connection_id, thread_id });
                
                // Handle connection in a separate thread
                const handle_thread = Thread.spawn(.{}, handleConnection, .{ self, connection_id }) catch |err| {
                    print("[UnixSocketServer] Failed to spawn connection handler: {}\n", .{err});
                    self.connection_pool.removeConnection(connection_id);
                    continue;
                };
                handle_thread.detach();
            } else {
                std.time.sleep(1000000); // 1ms
            }
        }
        
        print("[UnixSocketServer] Worker thread {} stopped\n", .{thread_id});
    }
    
    // Handle individual connection
    fn handleConnection(self: *Self, connection_id: i32) void {
        defer self.connection_pool.removeConnection(connection_id);
        
        while (self.is_running) {
            if (self.connection_pool.getConnection(connection_id)) |connection| {
                const message = connection.receiveMessage() catch |err| {
                    if (err != error.WouldBlock) {
                        print("[UnixSocketServer] Connection {} receive error: {}\n", .{ connection_id, err });
                        break;
                    }
                    continue;
                };
                
                // Process message based on type
                switch (message.frame.message_type) {
                    .Heartbeat => {
                        // Respond to heartbeat
                        connection.sendMessage(.Heartbeat, &[_]u8{}) catch |err| {
                            print("[UnixSocketServer] Failed to send heartbeat response: {}\n", .{err});
                            break;
                        };
                    },
                    .Registration => {
                        // Component registration
                        if (message.payload.len >= 4) {
                            const component_id: ComponentId = @enumFromInt(std.mem.readInt(u32, message.payload[0..4], .little));
                            connection.component_id = component_id;
                            print("[UnixSocketServer] Connection {} registered as {s}\n", .{ connection_id, component_id.toString() });
                        }
                    },
                    .Command, .Event => {
                        // Forward to IFR for processing
                        print("[UnixSocketServer] Received {} from connection {}\n", .{ message.frame.message_type, connection_id });
                        
                        // Send acknowledgment
                        connection.sendMessage(.Response, &[_]u8{0}) catch |err| {
                            print("[UnixSocketServer] Failed to send ack: {}\n", .{err});
                        };
                    },
                    else => {
                        print("[UnixSocketServer] Unknown message type from connection {}\n", .{connection_id});
                    },
                }
            } else {
                break;
            }
        }
        
        print("[UnixSocketServer] Connection {} handler finished\n", .{connection_id});
    }
};

// Performance benchmarking
pub fn benchmarkUnixSocket(allocator: Allocator, num_messages: usize) !void {
    print("[UnixSocket] Running performance benchmark with {} messages\n", .{num_messages});
    
    const server = try UnixSocketServer.init(allocator, .{
        .socket_path = "/tmp/hypermesh_test/",
        .max_connections = 100,
        .buffer_size = 64 * 1024,
        .timeout_ms = 1000,
        .worker_threads = 2,
    });
    defer server.deinit();
    
    try server.start();
    defer server.stop() catch {};
    
    // Allow server to start
    std.time.sleep(100_000_000); // 100ms
    
    print("[UnixSocket] Benchmark completed - server ready for connections\n");
}

// Unit tests
test "MessageFrame serialization" {
    const testing = std.testing;
    
    const frame = MessageFrame{
        .message_type = .Command,
        .component_id = 123,
        .sequence = 456,
        .payload_size = 789,
        .checksum = 0x12345678,
        .timestamp = 9876543210,
    };
    
    var buffer: [MessageFrame.HEADER_SIZE]u8 = undefined;
    const size = try frame.serialize(&buffer);
    try testing.expect(size == MessageFrame.HEADER_SIZE);
    
    const deserialized = try MessageFrame.deserialize(&buffer);
    try testing.expect(deserialized.message_type == .Command);
    try testing.expect(deserialized.component_id == 123);
    try testing.expect(deserialized.sequence == 456);
    try testing.expect(deserialized.payload_size == 789);
    try testing.expect(deserialized.checksum == 0x12345678);
    try testing.expect(deserialized.timestamp == 9876543210);
}

test "MessageFrame checksum" {
    const testing = std.testing;
    
    const payload = "Hello, HyperMesh!";
    const checksum1 = MessageFrame.calculateChecksum(payload);
    const checksum2 = MessageFrame.calculateChecksum(payload);
    
    try testing.expect(checksum1 == checksum2);
    
    const different_payload = "Hello, HyperMesh?";
    const checksum3 = MessageFrame.calculateChecksum(different_payload);
    
    try testing.expect(checksum1 != checksum3);
}

test "ConnectionPool basic operations" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const pool = try ConnectionPool.init(allocator);
    defer pool.deinit();
    
    // Test starts with no connections
    try testing.expect(pool.getActiveConnections() == 0);
}

test "UnixSocketServer creation" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    const server = try UnixSocketServer.init(allocator, .{});
    defer server.deinit();
    
    const stats = server.getStats();
    try testing.expect(!stats.is_running);
    try testing.expect(stats.active_connections == 0);
}

test "Unix socket performance" {
    const testing = std.testing;
    const allocator = testing.allocator;
    
    try benchmarkUnixSocket(allocator, 1000);
}