/*!
# Baseline Performance Generator

Dedicated tool for generating baseline performance measurements
without MFN optimizations for accurate comparison data.
*/

use clap::Parser;
use mfn_benchmarks::baseline::*;
use mfn_benchmarks::reporting::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "baseline-generator")]
#[command(about = "Generate baseline performance measurements for MFN comparison")]
#[command(version = mfn_benchmarks::VERSION)]
struct Cli {
    /// Number of flows to process
    #[arg(short, long, default_value = "10000")]
    flow_count: usize,

    /// Output directory
    #[arg(short, long, default_value = "./baseline_measurements")]
    output: PathBuf,

    /// Enable network call simulation
    #[arg(long)]
    simulate_network: bool,

    /// Enable database lookup simulation
    #[arg(long, default_value = "true")]
    simulate_database: bool,

    /// Enable ML inference simulation
    #[arg(long, default_value = "true")]
    simulate_ml: bool,

    /// Network latency in milliseconds
    #[arg(long, default_value = "2.0")]
    network_latency: f64,

    /// Database latency in milliseconds
    #[arg(long, default_value = "5.0")]
    database_latency: f64,

    /// ML inference latency in milliseconds
    #[arg(long, default_value = "50.0")]
    ml_latency: f64,

    /// Error rate percentage
    #[arg(long, default_value = "1.0")]
    error_rate: f64,

    /// Generate HTML report
    #[arg(long)]
    generate_report: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    tracing_subscriber::fmt::init();

    println!("🔧 MFN Baseline Performance Generator v{}", mfn_benchmarks::VERSION);
    println!("══════════════════════════════════════════════════════════════");
    println!("Flow count: {}", cli.flow_count);
    println!("Output directory: {}", cli.output.display());
    println!("Network simulation: {}", cli.simulate_network);
    println!("Database simulation: {}", cli.simulate_database);
    println!("ML simulation: {}", cli.simulate_ml);
    
    // Create output directory
    std::fs::create_dir_all(&cli.output)?;

    // Configure baseline system
    let baseline_config = BaselineConfig {
        enable_network_calls: cli.simulate_network,
        simulate_database_lookups: cli.simulate_database,
        simulate_ml_inference: cli.simulate_ml,
        network_latency_ms: cli.network_latency,
        database_latency_ms: cli.database_latency,
        ml_inference_latency_ms: cli.ml_latency,
        error_rate_percent: cli.error_rate,
    };

    println!("\n🚀 Starting baseline measurement generation...");
    
    // Create baseline generator
    let mut generator = BaselineGenerator::new(baseline_config);
    
    // Generate baseline measurements
    let start_time = std::time::Instant::now();
    let baseline_results = generator.generate_baseline_measurements(cli.flow_count).await?;
    let generation_time = start_time.elapsed();
    
    println!("✅ Baseline generation completed in {:.2}s", generation_time.as_secs_f64());
    
    // Calculate statistics
    let total_flows = baseline_results.len();
    let successful_flows = baseline_results.iter().filter(|r| r.registration_success).count();
    let success_rate = (successful_flows as f64 / total_flows as f64) * 100.0;
    
    let avg_latency = if !baseline_results.is_empty() {
        baseline_results.iter()
            .map(|r| r.total_processing_time.as_secs_f64() * 1000.0)
            .sum::<f64>() / baseline_results.len() as f64
    } else {
        0.0
    };
    
    let avg_throughput = if generation_time.as_secs_f64() > 0.0 {
        successful_flows as f64 / generation_time.as_secs_f64()
    } else {
        0.0
    };

    // Print summary
    println!("\n📊 BASELINE MEASUREMENT SUMMARY");
    println!("════════════════════════════════════════════════");
    println!("Total flows processed: {}", total_flows);
    println!("Successful flows: {} ({:.1}%)", successful_flows, success_rate);
    println!("Average latency: {:.3} ms", avg_latency);
    println!("Average throughput: {:.0} flows/sec", avg_throughput);
    
    // Component-wise breakdown
    if !baseline_results.is_empty() {
        let avg_registration = baseline_results.iter()
            .map(|r| r.registration_time.as_secs_f64() * 1000.0)
            .sum::<f64>() / baseline_results.len() as f64;
        let avg_similarity = baseline_results.iter()
            .map(|r| r.similarity_time.as_secs_f64() * 1000.0)
            .sum::<f64>() / baseline_results.len() as f64;
        let avg_routing = baseline_results.iter()
            .map(|r| r.routing_time.as_secs_f64() * 1000.0)
            .sum::<f64>() / baseline_results.len() as f64;
        let avg_prediction = baseline_results.iter()
            .map(|r| r.prediction_time.as_secs_f64() * 1000.0)
            .sum::<f64>() / baseline_results.len() as f64;

        println!("\n📋 Component Breakdown:");
        println!("  Registration (L1): {:.3} ms", avg_registration);
        println!("  Similarity (L2):   {:.3} ms", avg_similarity);
        println!("  Routing (L3):      {:.3} ms", avg_routing);
        println!("  Prediction (L4):   {:.3} ms", avg_prediction);
        
        let total_components = avg_registration + avg_similarity + avg_routing + avg_prediction;
        println!("  Components total:  {:.3} ms", total_components);
        println!("  Overhead:          {:.3} ms", avg_latency - total_components);
    }

    // Network and resource usage summary
    if !baseline_results.is_empty() {
        let avg_network_calls = baseline_results.iter()
            .map(|r| r.network_calls as f64)
            .sum::<f64>() / baseline_results.len() as f64;
        let avg_db_lookups = baseline_results.iter()
            .map(|r| r.database_lookups as f64)
            .sum::<f64>() / baseline_results.len() as f64;
        let avg_ml_inferences = baseline_results.iter()
            .map(|r| r.ml_inferences as f64)
            .sum::<f64>() / baseline_results.len() as f64;

        println!("\n🌐 Resource Usage per Flow:");
        println!("  Network calls:     {:.1}", avg_network_calls);
        println!("  Database lookups:  {:.1}", avg_db_lookups);
        println!("  ML inferences:     {:.1}", avg_ml_inferences);
    }

    // Save baseline data
    let baseline_file = cli.output.join("baseline_results.json");
    let baseline_json = serde_json::to_string_pretty(&baseline_results)?;
    std::fs::write(&baseline_file, baseline_json)?;
    println!("\n💾 Baseline data saved to: {}", baseline_file.display());

    // Save summary statistics
    let summary = BaselineSummary {
        timestamp: chrono::Utc::now(),
        config: baseline_config,
        total_flows,
        successful_flows,
        success_rate,
        generation_time_seconds: generation_time.as_secs_f64(),
        avg_latency_ms: avg_latency,
        avg_throughput_flows_per_sec: avg_throughput,
        component_breakdown: ComponentBreakdown {
            registration_ms: baseline_results.first().map(|r| r.registration_time.as_secs_f64() * 1000.0).unwrap_or(0.0),
            similarity_ms: baseline_results.first().map(|r| r.similarity_time.as_secs_f64() * 1000.0).unwrap_or(0.0),
            routing_ms: baseline_results.first().map(|r| r.routing_time.as_secs_f64() * 1000.0).unwrap_or(0.0),
            prediction_ms: baseline_results.first().map(|r| r.prediction_time.as_secs_f64() * 1000.0).unwrap_or(0.0),
        },
    };

    let summary_file = cli.output.join("baseline_summary.json");
    let summary_json = serde_json::to_string_pretty(&summary)?;
    std::fs::write(&summary_file, summary_json)?;
    println!("📄 Summary saved to: {}", summary_file.display());

    // Generate HTML report if requested
    if cli.generate_report {
        println!("\n📝 Generating baseline report...");
        generate_baseline_report(&cli, &baseline_results, &summary).await?;
    }

    println!("\n✅ Baseline generation completed successfully!");
    println!("════════════════════════════════════════════════");

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BaselineSummary {
    timestamp: chrono::DateTime<chrono::Utc>,
    config: BaselineConfig,
    total_flows: usize,
    successful_flows: usize,
    success_rate: f64,
    generation_time_seconds: f64,
    avg_latency_ms: f64,
    avg_throughput_flows_per_sec: f64,
    component_breakdown: ComponentBreakdown,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ComponentBreakdown {
    registration_ms: f64,
    similarity_ms: f64,
    routing_ms: f64,
    prediction_ms: f64,
}

async fn generate_baseline_report(
    cli: &Cli,
    baseline_results: &[BaselineFlowResult],
    summary: &BaselineSummary,
) -> anyhow::Result<()> {
    let report_file = cli.output.join("baseline_report.html");
    
    let html_content = format!(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>MFN Baseline Performance Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }}
        .header {{ text-align: center; border-bottom: 2px solid #333; padding-bottom: 20px; }}
        .summary {{ background: #f5f5f5; padding: 20px; border-radius: 8px; margin: 20px 0; }}
        .metric {{ display: inline-block; margin: 10px 20px; text-align: center; }}
        .metric-value {{ font-size: 2em; font-weight: bold; color: #2c3e50; }}
        .metric-label {{ color: #7f8c8d; }}
        .breakdown {{ margin: 30px 0; }}
        .breakdown table {{ width: 100%; border-collapse: collapse; }}
        .breakdown th, .breakdown td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        .breakdown th {{ background-color: #34495e; color: white; }}
        .chart {{ width: 100%; height: 300px; background: #ecf0f1; margin: 20px 0; padding: 20px; border-radius: 8px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🔧 MFN Baseline Performance Report</h1>
        <p>Generated: {}</p>
        <p>Flows Processed: {} | Success Rate: {:.1}%</p>
    </div>

    <div class="summary">
        <h2>📊 Performance Summary</h2>
        <div class="metric">
            <div class="metric-value">{:.3}</div>
            <div class="metric-label">Average Latency (ms)</div>
        </div>
        <div class="metric">
            <div class="metric-value">{:.0}</div>
            <div class="metric-label">Throughput (flows/sec)</div>
        </div>
        <div class="metric">
            <div class="metric-value">{:.1}%</div>
            <div class="metric-label">Success Rate</div>
        </div>
    </div>

    <div class="breakdown">
        <h2>📋 Component Performance Breakdown</h2>
        <table>
            <thead>
                <tr>
                    <th>Component</th>
                    <th>Layer</th>
                    <th>Average Latency (ms)</th>
                    <th>Percentage of Total</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Flow Registration</td>
                    <td>Layer 1 (IFR)</td>
                    <td>{:.3}</td>
                    <td>{:.1}%</td>
                </tr>
                <tr>
                    <td>Similarity Detection</td>
                    <td>Layer 2 (DSR)</td>
                    <td>{:.3}</td>
                    <td>{:.1}%</td>
                </tr>
                <tr>
                    <td>Route Finding</td>
                    <td>Layer 3 (ALM)</td>
                    <td>{:.3}</td>
                    <td>{:.1}%</td>
                </tr>
                <tr>
                    <td>Pattern Prediction</td>
                    <td>Layer 4 (CPE)</td>
                    <td>{:.3}</td>
                    <td>{:.1}%</td>
                </tr>
            </tbody>
        </table>
    </div>

    <div class="breakdown">
        <h2>🌐 Resource Usage Configuration</h2>
        <table>
            <thead>
                <tr>
                    <th>Resource Type</th>
                    <th>Enabled</th>
                    <th>Latency (ms)</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Network Calls</td>
                    <td>{}</td>
                    <td>{:.1}</td>
                </tr>
                <tr>
                    <td>Database Lookups</td>
                    <td>{}</td>
                    <td>{:.1}</td>
                </tr>
                <tr>
                    <td>ML Inference</td>
                    <td>{}</td>
                    <td>{:.1}</td>
                </tr>
            </tbody>
        </table>
    </div>

    <div class="chart">
        <h3>💡 Performance Insights</h3>
        <p>This baseline represents the performance of a traditional system without MFN optimizations:</p>
        <ul>
            <li><strong>Network calls</strong> simulate remote service dependencies</li>
            <li><strong>Database lookups</strong> represent persistent storage access</li>
            <li><strong>ML inference</strong> simulates machine learning model execution</li>
        </ul>
        <p>The MFN system aims to achieve significant improvements over these baseline measurements through:</p>
        <ul>
            <li>Local coordination via Unix sockets (Layer 1)</li>
            <li>Neural network optimization (Layer 2)</li>
            <li>Adaptive routing algorithms (Layer 3)</li>
            <li>Context prediction (Layer 4)</li>
        </ul>
    </div>

    <div style="text-align: center; margin-top: 40px; border-top: 1px solid #ddd; padding-top: 20px; color: #7f8c8d;">
        <p>Generated by MFN Baseline Generator v{}</p>
        <p>Total generation time: {:.2} seconds</p>
    </div>
</body>
</html>"#,
        summary.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
        summary.total_flows,
        summary.success_rate,
        summary.avg_latency_ms,
        summary.avg_throughput_flows_per_sec,
        summary.success_rate,
        summary.component_breakdown.registration_ms,
        (summary.component_breakdown.registration_ms / summary.avg_latency_ms) * 100.0,
        summary.component_breakdown.similarity_ms,
        (summary.component_breakdown.similarity_ms / summary.avg_latency_ms) * 100.0,
        summary.component_breakdown.routing_ms,
        (summary.component_breakdown.routing_ms / summary.avg_latency_ms) * 100.0,
        summary.component_breakdown.prediction_ms,
        (summary.component_breakdown.prediction_ms / summary.avg_latency_ms) * 100.0,
        if summary.config.enable_network_calls { "✅" } else { "❌" },
        summary.config.network_latency_ms,
        if summary.config.simulate_database_lookups { "✅" } else { "❌" },
        summary.config.database_latency_ms,
        if summary.config.simulate_ml_inference { "✅" } else { "❌" },
        summary.config.ml_inference_latency_ms,
        mfn_benchmarks::VERSION,
        summary.generation_time_seconds,
    );

    std::fs::write(&report_file, html_content)?;
    println!("📄 HTML report saved to: {}", report_file.display());

    Ok(())
}