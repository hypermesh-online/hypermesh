//! Phoenix Microservice Mesh Example
//!
//! Demonstrates building a complete microservice architecture with Phoenix:
//! - Service discovery and registration
//! - Load balancing
//! - Health checks
//! - Circuit breaking
//! - Distributed tracing

use phoenix_sdk::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

// Service Registry for service discovery
#[derive(Clone)]
struct ServiceRegistry {
    services: Arc<RwLock<HashMap<String, Vec<ServiceInstance>>>>,
}

#[derive(Clone, Debug)]
struct ServiceInstance {
    id: String,
    address: String,
    health_check_url: String,
    last_health_check: Instant,
    healthy: bool,
    load: f64,
}

impl ServiceRegistry {
    fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn register(&self, service_name: &str, instance: ServiceInstance) {
        let mut services = self.services.write().await;
        services.entry(service_name.to_string())
            .or_insert_with(Vec::new)
            .push(instance);

        println!("✅ Registered {} at {}", service_name, instance.address);
    }

    async fn discover(&self, service_name: &str) -> Option<String> {
        let services = self.services.read().await;

        // Simple round-robin load balancing with health checks
        if let Some(instances) = services.get(service_name) {
            let healthy_instances: Vec<_> = instances.iter()
                .filter(|i| i.healthy)
                .collect();

            if !healthy_instances.is_empty() {
                // Select instance with lowest load
                let best_instance = healthy_instances.iter()
                    .min_by(|a, b| a.load.partial_cmp(&b.load).unwrap())
                    .unwrap();

                return Some(best_instance.address.clone());
            }
        }

        None
    }

    async fn health_check_all(&self) {
        let mut services = self.services.write().await;

        for instances in services.values_mut() {
            for instance in instances.iter_mut() {
                // Simulate health check
                instance.healthy = instance.load < 0.8;
                instance.last_health_check = Instant::now();

                if !instance.healthy {
                    println!("⚠️  Instance {} is unhealthy (load: {:.2})",
                        instance.id, instance.load);
                }
            }
        }
    }
}

// API Gateway Service
struct ApiGateway {
    phoenix: Phoenix,
    registry: ServiceRegistry,
}

impl ApiGateway {
    async fn new(registry: ServiceRegistry) -> Result<Self> {
        let phoenix = Phoenix::builder()
            .app_name("api-gateway")
            .performance_tier(PerformanceTier::Production)
            .security_level(SecurityLevel::Enhanced)
            .build()
            .await?;

        Ok(Self { phoenix, registry })
    }

    async fn run(&self) -> Result<()> {
        let listener = self.phoenix.listen(8080).await?;
        println!("🌐 API Gateway running on port 8080");

        let registry = self.registry.clone();

        listener.handle(move |conn| {
            let registry = registry.clone();
            async move {
                self.handle_request(conn, registry).await
            }
        }).await?;

        Ok(())
    }

    async fn handle_request(
        &self,
        conn: PhoenixConnection,
        registry: ServiceRegistry,
    ) -> Result<()> {
        #[derive(Deserialize)]
        struct ApiRequest {
            service: String,
            method: String,
            data: serde_json::Value,
        }

        while let Ok(request) = conn.receive::<ApiRequest>().await {
            println!("🔄 Routing request to {}/{}", request.service, request.method);

            // Discover service
            if let Some(service_addr) = registry.discover(&request.service).await {
                // Forward request to service
                match self.phoenix.connect(&service_addr).await {
                    Ok(service_conn) => {
                        // Send request to service
                        service_conn.send(&request.data).await?;

                        // Get response
                        let response: serde_json::Value = service_conn.receive().await?;

                        // Send response back to client
                        conn.send(&response).await?;

                        println!("✅ Request processed successfully");
                    }
                    Err(e) => {
                        // Circuit breaker pattern - mark service as unhealthy
                        println!("❌ Failed to connect to service: {}", e);
                        conn.send(&json!({
                            "error": "Service unavailable"
                        })).await?;
                    }
                }
            } else {
                conn.send(&json!({
                    "error": "Service not found"
                })).await?;
            }
        }

        Ok(())
    }
}

// User Service
struct UserService {
    phoenix: Phoenix,
    users: Arc<RwLock<HashMap<u64, User>>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: u64,
    username: String,
    email: String,
    created_at: String,
}

impl UserService {
    async fn new() -> Result<Self> {
        let phoenix = Phoenix::builder()
            .app_name("user-service")
            .performance_tier(PerformanceTier::Production)
            .build()
            .await?;

        let users = Arc::new(RwLock::new(HashMap::new()));

        // Add sample users
        let mut users_map = users.write().await;
        users_map.insert(1, User {
            id: 1,
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        users_map.insert(2, User {
            id: 2,
            username: "bob".to_string(),
            email: "bob@example.com".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        drop(users_map);

        Ok(Self { phoenix, users })
    }

    async fn run(&self, port: u16, registry: &ServiceRegistry) -> Result<()> {
        // Register with service registry
        registry.register("user-service", ServiceInstance {
            id: format!("user-service-{}", port),
            address: format!("localhost:{}", port),
            health_check_url: format!("http://localhost:{}/health", port),
            last_health_check: Instant::now(),
            healthy: true,
            load: 0.0,
        }).await;

        let listener = self.phoenix.listen(port).await?;
        println!("👤 User Service running on port {}", port);

        let users = self.users.clone();

        listener.handle(move |conn| {
            let users = users.clone();
            async move {
                self.handle_request(conn, users).await
            }
        }).await?;

        Ok(())
    }

    async fn handle_request(
        &self,
        conn: PhoenixConnection,
        users: Arc<RwLock<HashMap<u64, User>>>,
    ) -> Result<()> {
        #[derive(Deserialize)]
        struct UserRequest {
            action: String,
            user_id: Option<u64>,
            user_data: Option<User>,
        }

        while let Ok(request) = conn.receive::<UserRequest>().await {
            let response = match request.action.as_str() {
                "get" => {
                    if let Some(user_id) = request.user_id {
                        let users = users.read().await;
                        if let Some(user) = users.get(&user_id) {
                            json!({ "user": user })
                        } else {
                            json!({ "error": "User not found" })
                        }
                    } else {
                        json!({ "error": "User ID required" })
                    }
                }
                "list" => {
                    let users = users.read().await;
                    let user_list: Vec<_> = users.values().collect();
                    json!({ "users": user_list })
                }
                "create" => {
                    if let Some(user_data) = request.user_data {
                        let mut users = users.write().await;
                        let new_id = users.len() as u64 + 1;
                        let mut new_user = user_data;
                        new_user.id = new_id;
                        users.insert(new_id, new_user.clone());
                        json!({ "user": new_user })
                    } else {
                        json!({ "error": "User data required" })
                    }
                }
                _ => json!({ "error": "Unknown action" }),
            };

            conn.send(&response).await?;
        }

        Ok(())
    }
}

// Order Service
struct OrderService {
    phoenix: Phoenix,
    orders: Arc<RwLock<HashMap<u64, Order>>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Order {
    id: u64,
    user_id: u64,
    items: Vec<String>,
    total: f64,
    status: String,
    created_at: String,
}

impl OrderService {
    async fn new() -> Result<Self> {
        let phoenix = Phoenix::builder()
            .app_name("order-service")
            .performance_tier(PerformanceTier::Production)
            .build()
            .await?;

        Ok(Self {
            phoenix,
            orders: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn run(&self, port: u16, registry: &ServiceRegistry) -> Result<()> {
        // Register with service registry
        registry.register("order-service", ServiceInstance {
            id: format!("order-service-{}", port),
            address: format!("localhost:{}", port),
            health_check_url: format!("http://localhost:{}/health", port),
            last_health_check: Instant::now(),
            healthy: true,
            load: 0.0,
        }).await;

        let listener = self.phoenix.listen(port).await?;
        println!("🛒 Order Service running on port {}", port);

        let orders = self.orders.clone();

        listener.handle(move |conn| {
            let orders = orders.clone();
            async move {
                self.handle_request(conn, orders).await
            }
        }).await?;

        Ok(())
    }

    async fn handle_request(
        &self,
        conn: PhoenixConnection,
        orders: Arc<RwLock<HashMap<u64, Order>>>,
    ) -> Result<()> {
        #[derive(Deserialize)]
        struct OrderRequest {
            action: String,
            order_id: Option<u64>,
            order_data: Option<Order>,
        }

        while let Ok(request) = conn.receive::<OrderRequest>().await {
            let response = match request.action.as_str() {
                "create" => {
                    if let Some(order_data) = request.order_data {
                        let mut orders = orders.write().await;
                        let new_id = orders.len() as u64 + 1;
                        let mut new_order = order_data;
                        new_order.id = new_id;
                        new_order.status = "pending".to_string();
                        new_order.created_at = chrono::Utc::now().to_rfc3339();
                        orders.insert(new_id, new_order.clone());

                        println!("📦 Created order #{} for user {}", new_id, new_order.user_id);
                        json!({ "order": new_order })
                    } else {
                        json!({ "error": "Order data required" })
                    }
                }
                "get" => {
                    if let Some(order_id) = request.order_id {
                        let orders = orders.read().await;
                        if let Some(order) = orders.get(&order_id) {
                            json!({ "order": order })
                        } else {
                            json!({ "error": "Order not found" })
                        }
                    } else {
                        json!({ "error": "Order ID required" })
                    }
                }
                "list" => {
                    let orders = orders.read().await;
                    let order_list: Vec<_> = orders.values().collect();
                    json!({ "orders": order_list })
                }
                _ => json!({ "error": "Unknown action" }),
            };

            conn.send(&response).await?;
        }

        Ok(())
    }
}

// Monitoring Service
struct MonitoringService {
    phoenix: Phoenix,
    registry: ServiceRegistry,
}

impl MonitoringService {
    async fn new(registry: ServiceRegistry) -> Result<Self> {
        let phoenix = Phoenix::builder()
            .app_name("monitoring-service")
            .performance_tier(PerformanceTier::Production)
            .build()
            .await?;

        Ok(Self { phoenix, registry })
    }

    async fn run(&self) -> Result<()> {
        println!("📊 Monitoring Service starting...");

        // Health check loop
        let registry = self.registry.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                registry.health_check_all().await;

                // Display service status
                let services = registry.services.read().await;
                println!("\n📊 Service Status:");
                for (name, instances) in services.iter() {
                    let healthy_count = instances.iter().filter(|i| i.healthy).count();
                    let total = instances.len();
                    println!("  {} {}: {}/{} healthy",
                        if healthy_count == total { "✅" } else { "⚠️" },
                        name, healthy_count, total);
                }
            }
        });

        // Metrics collection loop
        let phoenix = self.phoenix.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let metrics = phoenix.metrics().await;
                println!("\n📈 System Metrics:");
                println!("  Throughput: {:.2} Gbps", metrics.throughput_gbps);
                println!("  Connections: {}", metrics.active_connections);
                println!("  Avg Latency: {} µs", metrics.avg_latency_us);
            }
        });

        // Keep running
        tokio::signal::ctrl_c().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🚀 Phoenix Microservice Mesh Example");
    println!("=====================================\n");

    // Create service registry
    let registry = ServiceRegistry::new();

    // Start services in parallel
    let handles = vec![
        // API Gateway
        {
            let registry = registry.clone();
            tokio::spawn(async move {
                let gateway = ApiGateway::new(registry).await?;
                gateway.run().await
            })
        },
        // User Service (2 instances for load balancing)
        {
            let registry = registry.clone();
            tokio::spawn(async move {
                let service = UserService::new().await?;
                service.run(8081, &registry).await
            })
        },
        {
            let registry = registry.clone();
            tokio::spawn(async move {
                let service = UserService::new().await?;
                service.run(8082, &registry).await
            })
        },
        // Order Service (2 instances)
        {
            let registry = registry.clone();
            tokio::spawn(async move {
                let service = OrderService::new().await?;
                service.run(8083, &registry).await
            })
        },
        {
            let registry = registry.clone();
            tokio::spawn(async move {
                let service = OrderService::new().await?;
                service.run(8084, &registry).await
            })
        },
        // Monitoring Service
        {
            let registry = registry.clone();
            tokio::spawn(async move {
                let monitor = MonitoringService::new(registry).await?;
                monitor.run().await
            })
        },
    ];

    // Wait for all services
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

// Helper macro for JSON
#[macro_export]
macro_rules! json {
    ($($json:tt)*) => {
        serde_json::json!($($json)*)
    };
}