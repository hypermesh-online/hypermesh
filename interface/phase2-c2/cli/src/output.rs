//! Output formatting and display utilities

use anyhow::Result;
use colored::*;
use tabled::{Table, Tabled, settings::{Style, Color, object::Rows}};

use crate::cluster::{Cluster, Node};
use crate::service::Service;
use crate::node::{NodeInfo, NodeDetail, NodeResourceUsage};
use crate::debug::{ClusterEvent, PodResourceUsage};

/// Display system status information
pub fn display_status(status: &super::SystemStatus, format: &str) -> Result<()> {
    match format {
        "json" => {
            // Convert to JSON-serializable structure
            let json_status = serde_json::json!({
                "cluster_health": status.cluster_health,
                "node_count": status.node_count,
                "service_count": status.service_count,
                "active_connections": status.active_connections,
                "uptime_seconds": status.uptime.num_seconds(),
                "version": status.version,
                "components": status.components
            });
            println!("{}", serde_json::to_string_pretty(&json_status)?);
        },
        "yaml" => {
            let yaml_status = serde_json::json!({
                "cluster_health": status.cluster_health,
                "node_count": status.node_count,
                "service_count": status.service_count,
                "active_connections": status.active_connections,
                "uptime_seconds": status.uptime.num_seconds(),
                "version": status.version,
                "components": status.components
            });
            println!("{}", serde_yaml::to_string(&yaml_status)?);
        },
        _ => {
            // Table format (default)
            println!("  {} {}", "Health:".bright_white(), 
                     format_health_status(&status.cluster_health));
            println!("  {} {}", "Nodes:".bright_white(), 
                     status.node_count.to_string().bright_cyan());
            println!("  {} {}", "Services:".bright_white(), 
                     status.service_count.to_string().bright_cyan());
            println!("  {} {}", "Connections:".bright_white(), 
                     status.active_connections.to_string().bright_cyan());
            println!("  {} {}", "Uptime:".bright_white(), 
                     format_duration(&status.uptime).bright_cyan());
            println!("  {} {}", "Version:".bright_white(), 
                     status.version.bright_cyan());

            if let Some(components) = &status.components {
                println!();
                println!("{}", "Components:".bright_white().bold());
                
                let component_rows: Vec<ComponentRow> = components.iter().map(|c| {
                    ComponentRow {
                        name: c.name.clone(),
                        status: c.status.clone(),
                        connections: c.connections.to_string(),
                    }
                }).collect();

                let mut table = Table::new(component_rows);
                table.with(Style::rounded());
                println!("{}", table);
            }
        }
    }

    Ok(())
}

/// Display cluster information
pub fn display_cluster(cluster: &Cluster, format: &str) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(cluster)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(cluster)?);
        },
        _ => {
            // Table format (default)
            println!();
            println!("  {} {}", "Name:".bright_white(), cluster.name.bright_cyan());
            println!("  {} {}", "Status:".bright_white(), format_status(&cluster.status));
            println!("  {} {}", "Nodes:".bright_white(), cluster.node_count.to_string().bright_cyan());
            println!("  {} {}", "Version:".bright_white(), cluster.version.bright_cyan());
            println!("  {} {}", "Created:".bright_white(), 
                     cluster.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string().bright_cyan());
            println!("  {} {}", "Endpoint:".bright_white(), cluster.endpoint.bright_blue());
            println!("  {} {}", "HA:".bright_white(), 
                     if cluster.high_availability { "enabled".bright_green() } else { "disabled".dimmed() });

            if !cluster.nodes.is_empty() {
                println!();
                println!("{}", "Nodes:".bright_white().bold());
                
                let node_rows: Vec<NodeRow> = cluster.nodes.iter().map(|n| {
                    NodeRow {
                        id: n.id.clone(),
                        status: n.status.clone(),
                        cpu: format!("{:.1}%", n.cpu_usage),
                        memory: format!("{:.1}%", n.memory_usage),
                        disk: format!("{:.1}%", n.disk_usage),
                    }
                }).collect();

                let mut table = Table::new(node_rows);
                table.with(Style::rounded());
                println!("{}", table);
            }
        }
    }

    Ok(())
}

/// Display multiple clusters
pub fn display_clusters(clusters: &[Cluster], detailed: bool, format: &str) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(clusters)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(clusters)?);
        },
        _ => {
            // Table format (default)
            if clusters.is_empty() {
                println!("{}", "No clusters found.".dimmed());
                return Ok(());
            }

            let cluster_rows: Vec<ClusterRow> = clusters.iter().map(|c| {
                ClusterRow {
                    name: c.name.clone(),
                    status: c.status.clone(),
                    nodes: c.node_count.to_string(),
                    version: c.version.clone(),
                    created: c.created_at.format("%Y-%m-%d").to_string(),
                    ha: if c.high_availability { "Yes".to_string() } else { "No".to_string() },
                }
            }).collect();

            let mut table = Table::new(cluster_rows);
            table.with(Style::rounded());
            
            // Color the status column
            for i in 0..clusters.len() {
                let status_color = match clusters[i].status.as_str() {
                    "Running" => Color::FG_GREEN,
                    "Pending" => Color::FG_YELLOW,
                    "Failed" => Color::FG_RED,
                    _ => Color::FG_WHITE,
                };
                // table.modify(Rows::single(i + 1), status_color); // API changed
            }
            
            println!("{}", table);

            if detailed {
                println!();
                for cluster in clusters {
                    println!("{}", format!("--- {} ---", cluster.name).bright_blue());
                    display_cluster(cluster, "table")?;
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// Display service information
pub fn display_service(service: &Service, format: &str) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(service)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(service)?);
        },
        _ => {
            // Table format (default)
            println!();
            println!("  {} {}", "Name:".bright_white(), service.name.bright_cyan());
            println!("  {} {}", "Image:".bright_white(), service.image.bright_cyan());
            println!("  {} {}", "Status:".bright_white(), format_status(&service.status));
            println!("  {} {}", "Replicas:".bright_white(), 
                     format!("{}/{}", service.ready_replicas, service.replicas).bright_cyan());
            println!("  {} {}", "Created:".bright_white(), 
                     service.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string().bright_cyan());
            
            if let Some(endpoint) = &service.endpoint {
                println!("  {} {}", "Endpoint:".bright_white(), endpoint.bright_blue());
            }

            if !service.environment.is_empty() {
                println!("  {} {} variables", "Environment:".bright_white(), 
                         service.environment.len().to_string().bright_cyan());
            }

            // Resource usage
            println!();
            println!("  {} {}", "CPU:".bright_white(), 
                     format!("{:.1}%", service.cpu_usage).bright_cyan());
            println!("  {} {}", "Memory:".bright_white(), 
                     format!("{:.1} MB", service.memory_usage).bright_cyan());
            println!("  {} {}", "Network:".bright_white(), 
                     format!("↑ {} ↓ {} KB", service.network_tx / 1024, service.network_rx / 1024).bright_cyan());
        }
    }

    Ok(())
}

/// Display multiple services
pub fn display_services(services: &[Service], detailed: bool, format: &str) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(services)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(services)?);
        },
        _ => {
            // Table format (default)
            if services.is_empty() {
                println!("{}", "No services found.".dimmed());
                return Ok(());
            }

            let service_rows: Vec<ServiceRow> = services.iter().map(|s| {
                ServiceRow {
                    name: s.name.clone(),
                    image: s.image.clone(),
                    status: s.status.clone(),
                    replicas: format!("{}/{}", s.ready_replicas, s.replicas),
                    cpu: format!("{:.1}%", s.cpu_usage),
                    memory: format!("{:.0}MB", s.memory_usage),
                    created: s.created_at.format("%Y-%m-%d").to_string(),
                }
            }).collect();

            let mut table = Table::new(service_rows);
            table.with(Style::rounded());
            
            // Color the status column - disabled for now due to API changes
            // Color styling disabled for now due to API changes
            
            println!("{}", table);

            if detailed {
                println!();
                for service in services {
                    println!("{}", format!("--- {} ---", service.name).bright_blue());
                    display_service(service, "table")?;
                    println!();
                }
            }
        }
    }

    Ok(())
}

// Helper functions

fn format_health_status(status: &str) -> ColoredString {
    match status {
        "Healthy" => status.bright_green(),
        "Warning" => status.bright_yellow(),
        "Critical" => status.bright_red(),
        _ => status.normal(),
    }
}

fn format_status(status: &str) -> ColoredString {
    match status {
        "Running" => status.bright_green(),
        "Pending" => status.bright_yellow(),
        "Updating" => status.bright_blue(),
        "Failed" | "Error" => status.bright_red(),
        _ => status.normal(),
    }
}

fn format_duration(duration: &chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let days = total_seconds / (24 * 3600);
    let hours = (total_seconds % (24 * 3600)) / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

// Table row structures

#[derive(Tabled)]
struct ComponentRow {
    #[tabled(rename = "Component")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Connections")]
    connections: String,
}

#[derive(Tabled)]
struct NodeRow {
    #[tabled(rename = "Node ID")]
    id: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "CPU")]
    cpu: String,
    #[tabled(rename = "Memory")]
    memory: String,
    #[tabled(rename = "Disk")]
    disk: String,
}

#[derive(Tabled)]
struct ClusterRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Nodes")]
    nodes: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Created")]
    created: String,
    #[tabled(rename = "HA")]
    ha: String,
}

#[derive(Tabled)]
struct ServiceRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Image")]
    image: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Replicas")]
    replicas: String,
    #[tabled(rename = "CPU")]
    cpu: String,
    #[tabled(rename = "Memory")]
    memory: String,
    #[tabled(rename = "Created")]
    created: String,
}

#[derive(Tabled)]
struct NodeListRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "STATUS")]
    status: String,
    #[tabled(rename = "ROLES")]
    roles: String,
    #[tabled(rename = "AGE")]
    age: String,
    #[tabled(rename = "VERSION")]
    version: String,
    #[tabled(rename = "INTERNAL-IP")]
    internal_ip: String,
    #[tabled(rename = "EXTERNAL-IP")]
    external_ip: String,
}

#[derive(Tabled)]
struct NodeTopRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "CPU%")]
    cpu_percent: String,
    #[tabled(rename = "MEMORY")]
    memory_usage: String,
    #[tabled(rename = "MEMORY%")]
    memory_percent: String,
}

#[derive(Tabled)]
struct PodTopRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "CPU%")]
    cpu_percent: String,
    #[tabled(rename = "MEMORY")]
    memory_usage: String,
}

#[derive(Tabled)]
struct EventRow {
    #[tabled(rename = "TIME")]
    timestamp: String,
    #[tabled(rename = "TYPE")]
    type_: String,
    #[tabled(rename = "REASON")]
    reason: String,
    #[tabled(rename = "OBJECT")]
    object: String,
    #[tabled(rename = "MESSAGE")]
    message: String,
}

/// Display nodes list
pub fn display_nodes(
    nodes: &[NodeInfo],
    _show_resources: bool,
    _show_conditions: bool,
    format: &str,
) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(nodes)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(nodes)?);
        },
        _ => {
            let node_rows: Vec<NodeListRow> = nodes.iter().map(|n| {
                NodeListRow {
                    name: n.name.clone(),
                    status: n.status.clone(),
                    roles: n.roles.join(","),
                    age: format_duration(&n.age),
                    version: n.version.clone(),
                    internal_ip: n.internal_ip.clone(),
                    external_ip: n.external_ip.clone().unwrap_or_else(|| "<none>".to_string()),
                }
            }).collect();

            let mut table = Table::new(node_rows);
            table.with(Style::rounded());
            println!("{}", table);
        }
    }
    Ok(())
}

/// Display node details
pub fn display_node_detail(node: &NodeDetail, format: &str) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(node)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(node)?);
        },
        _ => {
            println!();
            println!("  {} {}", "Name:".bright_white(), node.name.bright_cyan());
            println!("  {} {}", "Status:".bright_white(), format_status(&node.status));
            println!("  {} {}", "Roles:".bright_white(), node.roles.join(",").bright_cyan());
            println!("  {} {}", "Created:".bright_white(), 
                     node.creation_timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string().bright_cyan());
            println!("  {} {}", "Internal IP:".bright_white(), node.internal_ip.bright_cyan());
            if let Some(external_ip) = &node.external_ip {
                println!("  {} {}", "External IP:".bright_white(), external_ip.bright_cyan());
            }
            println!("  {} {}", "OS Image:".bright_white(), node.os_image.bright_cyan());
            println!("  {} {}", "Kernel:".bright_white(), node.kernel_version.bright_cyan());
            println!("  {} {}", "Runtime:".bright_white(), node.container_runtime.bright_cyan());
            
            println!();
            println!("{}:", "Capacity".bright_white().bold());
            println!("  {} {}", "CPU:".bright_white(), node.cpu_capacity.bright_cyan());
            println!("  {} {}", "Memory:".bright_white(), node.memory_capacity.bright_cyan());
            println!("  {} {}", "Storage:".bright_white(), node.storage_capacity.bright_cyan());
            println!("  {} {}", "Pods:".bright_white(), node.pods_capacity.bright_cyan());
        }
    }
    Ok(())
}

/// Display node resource usage
pub fn display_node_top(
    nodes: &[NodeResourceUsage],
    no_headers: bool,
    format: &str,
) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(nodes)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(nodes)?);
        },
        _ => {
            let node_rows: Vec<NodeTopRow> = nodes.iter().map(|n| {
                NodeTopRow {
                    name: n.name.clone(),
                    cpu_percent: n.cpu_percent.clone(),
                    memory_usage: n.memory_usage.clone(),
                    memory_percent: n.memory_percent.clone(),
                }
            }).collect();

            let mut table = Table::new(node_rows);
            table.with(Style::rounded());
            if no_headers {
                // Header styling disabled for now
                // table.with(tabled::settings::Modify::new(Rows::first()).with(Color::new("\x1b[2m".to_string(), "\x1b[0m".to_string())));
            }
            println!("{}", table);
        }
    }
    Ok(())
}

/// Display pod resource usage
pub fn display_pod_top(
    pods: &[PodResourceUsage],
    no_headers: bool,
    format: &str,
) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(pods)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(pods)?);
        },
        _ => {
            let pod_rows: Vec<PodTopRow> = pods.iter().map(|p| {
                PodTopRow {
                    name: p.name.clone(),
                    namespace: p.namespace.clone(),
                    cpu_percent: p.cpu_percent.clone(),
                    memory_usage: p.memory_usage.clone(),
                }
            }).collect();

            let mut table = Table::new(pod_rows);
            table.with(Style::rounded());
            if no_headers {
                // Header styling disabled for now
                // table.with(tabled::settings::Modify::new(Rows::first()).with(Color::new("\x1b[2m".to_string(), "\x1b[0m".to_string())));
            }
            println!("{}", table);
        }
    }
    Ok(())
}

/// Display cluster events
pub fn display_events(
    events: &[ClusterEvent],
    format: &str,
) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(events)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(events)?);
        },
        _ => {
            let event_rows: Vec<EventRow> = events.iter().map(|e| {
                EventRow {
                    timestamp: e.timestamp.format("%H:%M:%S").to_string(),
                    type_: e.type_.clone(),
                    reason: e.reason.clone(),
                    object: e.object.clone(),
                    message: e.message.clone(),
                }
            }).collect();

            let mut table = Table::new(event_rows);
            table.with(Style::rounded());
            println!("{}", table);
        }
    }
    Ok(())
}