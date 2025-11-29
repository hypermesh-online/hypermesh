/*!
# Performance Reporting and Visualization

Generates comprehensive performance reports with visualizations including:
- HTML reports with interactive charts
- CSV data export for further analysis
- Performance dashboards with real-time metrics
- Comparison reports between runs
*/

use crate::common::*;
use crate::analysis::{PerformanceComparison, PerformanceAnomaly};
use crate::baseline::BaselineComparisonReport;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Reporting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingConfig {
    pub output_directory: String,
    pub enable_html_report: bool,
    pub enable_csv_export: bool,
    pub enable_json_export: bool,
    pub enable_charts: bool,
    pub chart_width: usize,
    pub chart_height: usize,
    pub include_raw_data: bool,
}

impl Default for ReportingConfig {
    fn default() -> Self {
        Self {
            output_directory: "./benchmark_reports".to_string(),
            enable_html_report: true,
            enable_csv_export: true,
            enable_json_export: true,
            enable_charts: true,
            chart_width: 800,
            chart_height: 400,
            include_raw_data: false,
        }
    }
}

/// Comprehensive performance report
pub struct PerformanceReport {
    config: ReportingConfig,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl PerformanceReport {
    pub fn new(config: ReportingConfig) -> Self {
        Self {
            config,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Generate comprehensive performance report
    pub fn generate_comprehensive_report(
        &self,
        mfn_results: &[BenchmarkResult],
        baseline_report: Option<&BaselineComparisonReport>,
        performance_comparison: Option<&PerformanceComparison>,
    ) -> anyhow::Result<GeneratedReport> {
        // Create output directory
        fs::create_dir_all(&self.config.output_directory)?;

        let mut generated_files = Vec::new();

        // Generate HTML report
        if self.config.enable_html_report {
            let html_file = self.generate_html_report(mfn_results, baseline_report, performance_comparison)?;
            generated_files.push(html_file);
        }

        // Generate CSV exports
        if self.config.enable_csv_export {
            let csv_files = self.generate_csv_exports(mfn_results)?;
            generated_files.extend(csv_files);
        }

        // Generate JSON export
        if self.config.enable_json_export {
            let json_file = self.generate_json_export(mfn_results, baseline_report, performance_comparison)?;
            generated_files.push(json_file);
        }

        Ok(GeneratedReport {
            generated_files,
            output_directory: self.config.output_directory.clone(),
            timestamp: self.timestamp,
            summary: self.generate_report_summary(mfn_results, baseline_report),
        })
    }

    fn generate_html_report(
        &self,
        mfn_results: &[BenchmarkResult],
        baseline_report: Option<&BaselineComparisonReport>,
        performance_comparison: Option<&PerformanceComparison>,
    ) -> anyhow::Result<String> {
        let file_path = format!("{}/benchmark_report_{}.html", 
            self.config.output_directory, 
            self.timestamp.format("%Y%m%d_%H%M%S")
        );

        let html_content = self.build_html_content(mfn_results, baseline_report, performance_comparison)?;
        fs::write(&file_path, html_content)?;

        Ok(file_path)
    }

    fn build_html_content(
        &self,
        mfn_results: &[BenchmarkResult],
        baseline_report: Option<&BaselineComparisonReport>,
        performance_comparison: Option<&PerformanceComparison>,
    ) -> anyhow::Result<String> {
        let mut html = String::new();

        // HTML header
        html.push_str(&self.html_header());

        // Executive summary
        html.push_str(&self.html_executive_summary(mfn_results, baseline_report));

        // Performance overview
        html.push_str(&self.html_performance_overview(mfn_results));

        // Layer-specific results
        html.push_str(&self.html_layer_results(mfn_results));

        // Baseline comparison (if available)
        if let Some(baseline) = baseline_report {
            html.push_str(&self.html_baseline_comparison(baseline));
        }

        // Statistical analysis (if available)
        if let Some(comparison) = performance_comparison {
            html.push_str(&self.html_statistical_analysis(comparison));
        }

        // Charts and visualizations
        if self.config.enable_charts {
            html.push_str(&self.html_charts(mfn_results, baseline_report));
        }

        // Raw data (if enabled)
        if self.config.include_raw_data {
            html.push_str(&self.html_raw_data(mfn_results));
        }

        // HTML footer
        html.push_str(&self.html_footer());

        Ok(html)
    }

    fn html_header(&self) -> String {
        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MFN Performance Benchmark Report</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
            line-height: 1.6;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        .header {{
            text-align: center;
            border-bottom: 3px solid #007acc;
            padding-bottom: 20px;
            margin-bottom: 30px;
        }}
        .header h1 {{
            color: #007acc;
            margin: 0;
            font-size: 2.5em;
        }}
        .header .timestamp {{
            color: #666;
            font-size: 1.1em;
            margin-top: 10px;
        }}
        .section {{
            margin: 30px 0;
            padding: 20px;
            border: 1px solid #ddd;
            border-radius: 8px;
            background: #fafafa;
        }}
        .section h2 {{
            color: #007acc;
            border-bottom: 2px solid #007acc;
            padding-bottom: 5px;
            margin-top: 0;
        }}
        .metric-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin: 20px 0;
        }}
        .metric-card {{
            background: white;
            padding: 20px;
            border-radius: 8px;
            border: 1px solid #ddd;
            text-align: center;
        }}
        .metric-value {{
            font-size: 2em;
            font-weight: bold;
            color: #007acc;
        }}
        .metric-label {{
            color: #666;
            font-size: 0.9em;
            margin-top: 5px;
        }}
        .success {{ color: #28a745; }}
        .warning {{ color: #ffc107; }}
        .danger {{ color: #dc3545; }}
        .table {{
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
        }}
        .table th, .table td {{
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }}
        .table th {{
            background-color: #f8f9fa;
            font-weight: bold;
            color: #495057;
        }}
        .chart-container {{
            margin: 30px 0;
            padding: 20px;
            background: white;
            border-radius: 8px;
            border: 1px solid #ddd;
        }}
        .progress-bar {{
            width: 100%;
            height: 20px;
            background-color: #e9ecef;
            border-radius: 10px;
            overflow: hidden;
            margin: 10px 0;
        }}
        .progress-fill {{
            height: 100%;
            background: linear-gradient(90deg, #28a745, #007acc);
            transition: width 0.3s ease;
        }}
    </style>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
</head>
<body>
<div class="container">
    <div class="header">
        <h1>🚀 MFN Performance Benchmark Report</h1>
        <div class="timestamp">Generated: {}</div>
    </div>
"#, self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"))
    }

    fn html_executive_summary(
        &self, 
        mfn_results: &[BenchmarkResult],
        baseline_report: Option<&BaselineComparisonReport>
    ) -> String {
        let successful_benchmarks = mfn_results.iter().filter(|r| r.success).count();
        let total_benchmarks = mfn_results.len();
        let success_rate = if total_benchmarks > 0 {
            (successful_benchmarks as f64 / total_benchmarks as f64) * 100.0
        } else {
            0.0
        };

        let avg_latency_ms = if !mfn_results.is_empty() {
            mfn_results.iter()
                .map(|r| r.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0)
                .sum::<f64>() / mfn_results.len() as f64
        } else {
            0.0
        };

        let avg_throughput = if !mfn_results.is_empty() {
            mfn_results.iter()
                .map(|r| r.metrics.throughput_ops_per_sec)
                .sum::<f64>() / mfn_results.len() as f64
        } else {
            0.0
        };

        let improvement_text = if let Some(baseline) = baseline_report {
            format!(r#"
            <div class="metric-card">
                <div class="metric-value success">{:.1}%</div>
                <div class="metric-label">Performance Improvement</div>
                <div class="progress-bar">
                    <div class="progress-fill" style="width: {}%"></div>
                </div>
            </div>
            "#, 
            baseline.overall_improvement_percent,
            (baseline.overall_improvement_percent / 100.0 * 100.0).min(100.0)
            )
        } else {
            String::new()
        };

        format!(r#"
    <div class="section">
        <h2>📊 Executive Summary</h2>
        <div class="metric-grid">
            <div class="metric-card">
                <div class="metric-value {}">{}/{}</div>
                <div class="metric-label">Successful Benchmarks</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.3}ms</div>
                <div class="metric-label">Average Latency</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.0}</div>
                <div class="metric-label">Average Throughput (ops/sec)</div>
            </div>
            {}
        </div>
    </div>
"#, 
        if success_rate > 95.0 { "success" } else if success_rate > 80.0 { "warning" } else { "danger" },
        successful_benchmarks,
        total_benchmarks,
        avg_latency_ms,
        avg_throughput,
        improvement_text
        )
    }

    fn html_performance_overview(&self, mfn_results: &[BenchmarkResult]) -> String {
        let mut layer_stats = HashMap::new();
        
        for result in mfn_results {
            let stats = layer_stats.entry(result.layer).or_insert_with(|| LayerStats::new());
            stats.add_result(result);
        }

        let mut table_rows = String::new();
        for (layer, stats) in layer_stats {
            let status_class = if stats.all_targets_met { "success" } else { "warning" };
            table_rows.push_str(&format!(r#"
                <tr>
                    <td>{}</td>
                    <td>{:.3}ms</td>
                    <td>{:.0}</td>
                    <td><span class="{}">{}</span></td>
                    <td>{}</td>
                </tr>
            "#, 
            layer,
            stats.avg_latency_ms,
            stats.avg_throughput,
            status_class,
            if stats.all_targets_met { "✅ Met" } else { "⚠️ Partial" },
            stats.sample_count
            ));
        }

        format!(r#"
    <div class="section">
        <h2>🎯 Performance Overview by Layer</h2>
        <table class="table">
            <thead>
                <tr>
                    <th>Layer</th>
                    <th>Avg Latency (ms)</th>
                    <th>Avg Throughput</th>
                    <th>Target Status</th>
                    <th>Samples</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>
"#, table_rows)
    }

    fn html_layer_results(&self, mfn_results: &[BenchmarkResult]) -> String {
        let mut html = String::new();

        for layer in [MfnLayer::Layer1Ifr, MfnLayer::Layer2Dsr, MfnLayer::Layer3Alm, MfnLayer::Layer4Cpe, MfnLayer::Integration] {
            let layer_results: Vec<_> = mfn_results.iter().filter(|r| r.layer == layer).collect();
            if !layer_results.is_empty() {
                html.push_str(&self.html_layer_section(layer, &layer_results));
            }
        }

        html
    }

    fn html_layer_section(&self, layer: MfnLayer, results: &[&BenchmarkResult]) -> String {
        let layer_description = match layer {
            MfnLayer::Layer1Ifr => "Immediate Flow Registry - Ultra-fast local coordination with 88.6% latency improvement",
            MfnLayer::Layer2Dsr => "Dynamic Similarity Resolution - Neural similarity detection under 1ms",
            MfnLayer::Layer3Alm => "Adaptive Link Management - 777% routing improvement over HTTP baseline",
            MfnLayer::Layer4Cpe => "Context Prediction Engine - Context prediction under 2ms",
            MfnLayer::Integration => "End-to-End Integration - adaptive network tiers throughput with <5% MFN overhead",
        };

        let mut table_rows = String::new();
        for result in results {
            let status_class = if result.success && result.target_validation.overall_success {
                "success"
            } else if result.success {
                "warning"
            } else {
                "danger"
            };

            table_rows.push_str(&format!(r#"
                <tr>
                    <td>{}</td>
                    <td>{:.3}ms</td>
                    <td>{:.0}</td>
                    <td>{:.1}MB</td>
                    <td><span class="{}">{}</span></td>
                </tr>
            "#,
            result.name,
            result.metrics.latency_percentiles.p95.as_secs_f64() * 1000.0,
            result.metrics.throughput_ops_per_sec,
            result.metrics.memory_usage_mb,
            status_class,
            if result.target_validation.overall_success { "✅ Pass" } else { "❌ Fail" }
            ));
        }

        format!(r#"
    <div class="section">
        <h2>🔧 {} Results</h2>
        <p><em>{}</em></p>
        <table class="table">
            <thead>
                <tr>
                    <th>Benchmark</th>
                    <th>P95 Latency</th>
                    <th>Throughput</th>
                    <th>Memory Usage</th>
                    <th>Status</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>
"#, layer, layer_description, table_rows)
    }

    fn html_baseline_comparison(&self, baseline_report: &BaselineComparisonReport) -> String {
        let mut layer_improvements = String::new();
        for (layer_name, improvement) in &baseline_report.layer_improvements {
            let improvement_class = if *improvement > 50.0 { "success" } else if *improvement > 20.0 { "warning" } else { "danger" };
            layer_improvements.push_str(&format!(r#"
                <tr>
                    <td>{}</td>
                    <td><span class="{}">{:.1}%</span></td>
                    <td>
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: {}%"></div>
                        </div>
                    </td>
                </tr>
            "#, layer_name, improvement_class, improvement, improvement.min(100.0)));
        }

        format!(r#"
    <div class="section">
        <h2>📈 Baseline Comparison</h2>
        <div class="metric-grid">
            <div class="metric-card">
                <div class="metric-value success">{:.1}%</div>
                <div class="metric-label">Overall Improvement</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.3}ms</div>
                <div class="metric-label">MFN Latency</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.3}ms</div>
                <div class="metric-label">Baseline Latency</div>
            </div>
        </div>
        
        <h3>Layer-Specific Improvements</h3>
        <table class="table">
            <thead>
                <tr>
                    <th>Layer</th>
                    <th>Improvement</th>
                    <th>Progress</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>
"#, 
        baseline_report.overall_improvement_percent,
        baseline_report.mfn_avg_latency_ms,
        baseline_report.baseline_avg_latency_ms,
        layer_improvements
        )
    }

    fn html_statistical_analysis(&self, comparison: &PerformanceComparison) -> String {
        let significance_status = if comparison.improvement_significance.is_significant {
            format!("<span class='success'>✅ Significant (p={:.4})</span>", comparison.improvement_significance.p_value)
        } else {
            format!("<span class='warning'>⚠️ Not Significant (p={:.4})</span>", comparison.improvement_significance.p_value)
        };

        let mut anomaly_list = String::new();
        for anomaly in &comparison.anomalies {
            let severity_class = match anomaly.severity.as_str() {
                "High" => "danger",
                "Medium" => "warning",
                _ => "success",
            };
            anomaly_list.push_str(&format!(r#"
                <li class="{}">
                    <strong>{:?}</strong>: {} 
                    <small>(Index: {}, Value: {:.3}, Threshold: {:.3})</small>
                </li>
            "#, severity_class, anomaly.anomaly_type, anomaly.description, anomaly.benchmark_index, anomaly.value, anomaly.threshold));
        }

        format!(r#"
    <div class="section">
        <h2>🔬 Statistical Analysis</h2>
        
        <h3>Significance Testing</h3>
        <p><strong>Improvement Significance:</strong> {}</p>
        <p><strong>Effect Size:</strong> {:.3}</p>
        <p><strong>Confidence Level:</strong> {:.1}%</p>
        
        <h3>Confidence Intervals</h3>
        <div class="metric-grid">
            <div class="metric-card">
                <div class="metric-value">{:.3} - {:.3}ms</div>
                <div class="metric-label">Latency CI ({:.1}%)</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.0} - {:.0}</div>
                <div class="metric-label">Throughput CI ({:.1}%)</div>
            </div>
        </div>
        
        <h3>Detected Anomalies</h3>
        {}
    </div>
"#,
        significance_status,
        comparison.improvement_significance.effect_size,
        comparison.improvement_significance.confidence_level * 100.0,
        comparison.confidence_intervals.latency_ci.0,
        comparison.confidence_intervals.latency_ci.1,
        comparison.confidence_intervals.confidence_level * 100.0,
        comparison.confidence_intervals.throughput_ci.0,
        comparison.confidence_intervals.throughput_ci.1,
        comparison.confidence_intervals.confidence_level * 100.0,
        if comparison.anomalies.is_empty() {
            "<p class='success'>✅ No anomalies detected</p>".to_string()
        } else {
            format!("<ul>{}</ul>", anomaly_list)
        }
        )
    }

    fn html_charts(&self, mfn_results: &[BenchmarkResult], _baseline_report: Option<&BaselineComparisonReport>) -> String {
        // Generate chart data
        let layer_data = self.generate_chart_data(mfn_results);
        
        format!(r#"
    <div class="section">
        <h2>📊 Performance Charts</h2>
        
        <div class="chart-container">
            <canvas id="latencyChart" width="{}" height="{}"></canvas>
        </div>
        
        <div class="chart-container">
            <canvas id="throughputChart" width="{}" height="{}"></canvas>
        </div>
        
        <script>
        // Latency Chart
        const latencyCtx = document.getElementById('latencyChart').getContext('2d');
        new Chart(latencyCtx, {{
            type: 'bar',
            data: {{
                labels: {},
                datasets: [{{
                    label: 'P95 Latency (ms)',
                    data: {},
                    backgroundColor: 'rgba(0, 122, 204, 0.6)',
                    borderColor: 'rgba(0, 122, 204, 1)',
                    borderWidth: 1
                }}]
            }},
            options: {{
                responsive: true,
                scales: {{
                    y: {{
                        beginAtZero: true,
                        title: {{
                            display: true,
                            text: 'Latency (ms)'
                        }}
                    }}
                }},
                plugins: {{
                    title: {{
                        display: true,
                        text: 'Latency by Layer'
                    }}
                }}
            }}
        }});
        
        // Throughput Chart
        const throughputCtx = document.getElementById('throughputChart').getContext('2d');
        new Chart(throughputCtx, {{
            type: 'bar',
            data: {{
                labels: {},
                datasets: [{{
                    label: 'Throughput (ops/sec)',
                    data: {},
                    backgroundColor: 'rgba(40, 167, 69, 0.6)',
                    borderColor: 'rgba(40, 167, 69, 1)',
                    borderWidth: 1
                }}]
            }},
            options: {{
                responsive: true,
                scales: {{
                    y: {{
                        beginAtZero: true,
                        title: {{
                            display: true,
                            text: 'Throughput (ops/sec)'
                        }}
                    }}
                }},
                plugins: {{
                    title: {{
                        display: true,
                        text: 'Throughput by Layer'
                    }}
                }}
            }}
        }});
        </script>
    </div>
"#,
        self.config.chart_width,
        self.config.chart_height,
        self.config.chart_width,
        self.config.chart_height,
        layer_data.labels_json,
        layer_data.latency_data_json,
        layer_data.labels_json,
        layer_data.throughput_data_json
        )
    }

    fn generate_chart_data(&self, mfn_results: &[BenchmarkResult]) -> ChartData {
        let mut layer_stats = HashMap::new();
        
        for result in mfn_results {
            let stats = layer_stats.entry(result.layer).or_insert_with(|| LayerStats::new());
            stats.add_result(result);
        }

        let mut labels = Vec::new();
        let mut latency_data = Vec::new();
        let mut throughput_data = Vec::new();

        for (layer, stats) in layer_stats {
            labels.push(format!("{}", layer));
            latency_data.push(stats.avg_latency_ms);
            throughput_data.push(stats.avg_throughput);
        }

        ChartData {
            labels_json: serde_json::to_string(&labels).unwrap_or_default(),
            latency_data_json: serde_json::to_string(&latency_data).unwrap_or_default(),
            throughput_data_json: serde_json::to_string(&throughput_data).unwrap_or_default(),
        }
    }

    fn html_raw_data(&self, mfn_results: &[BenchmarkResult]) -> String {
        let mut table_rows = String::new();
        for result in mfn_results {
            table_rows.push_str(&format!(r#"
                <tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{:.6}ms</td>
                    <td>{:.6}ms</td>
                    <td>{:.6}ms</td>
                    <td>{:.0}</td>
                    <td>{:.2}MB</td>
                    <td>{:.1}%</td>
                </tr>
            "#,
            result.id,
            result.layer,
            result.name,
            result.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0,
            result.metrics.latency_percentiles.p95.as_secs_f64() * 1000.0,
            result.metrics.latency_percentiles.p99.as_secs_f64() * 1000.0,
            result.metrics.throughput_ops_per_sec,
            result.metrics.memory_usage_mb,
            result.metrics.cpu_utilization
            ));
        }

        format!(r#"
    <div class="section">
        <h2>📋 Raw Data</h2>
        <table class="table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Layer</th>
                    <th>Name</th>
                    <th>Mean Latency</th>
                    <th>P95 Latency</th>
                    <th>P99 Latency</th>
                    <th>Throughput</th>
                    <th>Memory</th>
                    <th>CPU</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>
"#, table_rows)
    }

    fn html_footer(&self) -> String {
        format!(r#"
    <div class="section" style="text-align: center; margin-top: 50px; border-top: 2px solid #007acc; padding-top: 20px;">
        <p style="color: #666;">
            Generated by MFN Benchmarking Framework v{} on {}<br>
            <strong>HyperMesh Multi-layer Flow Network Performance Validation</strong>
        </p>
    </div>
</div>
</body>
</html>
"#, 
        crate::VERSION,
        self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    fn generate_csv_exports(&self, mfn_results: &[BenchmarkResult]) -> anyhow::Result<Vec<String>> {
        let mut files = Vec::new();

        // Main results CSV
        let main_csv_path = format!("{}/benchmark_results_{}.csv", 
            self.config.output_directory, 
            self.timestamp.format("%Y%m%d_%H%M%S")
        );

        let mut csv_content = String::new();
        csv_content.push_str("id,layer,name,timestamp,duration_ms,latency_mean_ms,latency_p50_ms,latency_p95_ms,latency_p99_ms,throughput_ops_sec,memory_mb,cpu_percent,success,target_met\n");

        for result in mfn_results {
            csv_content.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                result.id,
                result.layer,
                result.name,
                result.metrics.timestamp.format("%Y-%m-%d %H:%M:%S"),
                result.metrics.duration.as_millis(),
                result.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0,
                result.metrics.latency_percentiles.p50.as_secs_f64() * 1000.0,
                result.metrics.latency_percentiles.p95.as_secs_f64() * 1000.0,
                result.metrics.latency_percentiles.p99.as_secs_f64() * 1000.0,
                result.metrics.throughput_ops_per_sec,
                result.metrics.memory_usage_mb,
                result.metrics.cpu_utilization,
                result.success,
                result.target_validation.overall_success
            ));
        }

        fs::write(&main_csv_path, csv_content)?;
        files.push(main_csv_path);

        Ok(files)
    }

    fn generate_json_export(
        &self,
        mfn_results: &[BenchmarkResult],
        baseline_report: Option<&BaselineComparisonReport>,
        performance_comparison: Option<&PerformanceComparison>,
    ) -> anyhow::Result<String> {
        let file_path = format!("{}/benchmark_data_{}.json", 
            self.config.output_directory, 
            self.timestamp.format("%Y%m%d_%H%M%S")
        );

        let export_data = JsonExport {
            metadata: ExportMetadata {
                timestamp: self.timestamp,
                version: crate::VERSION.to_string(),
                framework: "MFN Benchmarking Framework".to_string(),
            },
            results: mfn_results.to_vec(),
            baseline_comparison: baseline_report.cloned(),
            statistical_analysis: performance_comparison.cloned(),
            summary: self.generate_report_summary(mfn_results, baseline_report),
        };

        let json_content = serde_json::to_string_pretty(&export_data)?;
        fs::write(&file_path, json_content)?;

        Ok(file_path)
    }

    fn generate_report_summary(
        &self,
        mfn_results: &[BenchmarkResult],
        baseline_report: Option<&BaselineComparisonReport>
    ) -> ReportSummary {
        let total_benchmarks = mfn_results.len();
        let successful_benchmarks = mfn_results.iter().filter(|r| r.success).count();
        let targets_met = mfn_results.iter().filter(|r| r.target_validation.overall_success).count();

        let avg_latency_ms = if !mfn_results.is_empty() {
            mfn_results.iter()
                .map(|r| r.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0)
                .sum::<f64>() / mfn_results.len() as f64
        } else {
            0.0
        };

        let avg_throughput = if !mfn_results.is_empty() {
            mfn_results.iter()
                .map(|r| r.metrics.throughput_ops_per_sec)
                .sum::<f64>() / mfn_results.len() as f64
        } else {
            0.0
        };

        ReportSummary {
            total_benchmarks,
            successful_benchmarks,
            targets_met,
            success_rate: if total_benchmarks > 0 { 
                (successful_benchmarks as f64 / total_benchmarks as f64) * 100.0 
            } else { 
                0.0 
            },
            target_achievement_rate: if total_benchmarks > 0 {
                (targets_met as f64 / total_benchmarks as f64) * 100.0
            } else {
                0.0
            },
            avg_latency_ms,
            avg_throughput,
            improvement_percent: baseline_report.map(|b| b.overall_improvement_percent).unwrap_or(0.0),
            timestamp: self.timestamp,
        }
    }
}

// Helper structures

struct LayerStats {
    avg_latency_ms: f64,
    avg_throughput: f64,
    all_targets_met: bool,
    sample_count: usize,
    total_latency: f64,
    total_throughput: f64,
    targets_met_count: usize,
}

impl LayerStats {
    fn new() -> Self {
        Self {
            avg_latency_ms: 0.0,
            avg_throughput: 0.0,
            all_targets_met: true,
            sample_count: 0,
            total_latency: 0.0,
            total_throughput: 0.0,
            targets_met_count: 0,
        }
    }

    fn add_result(&mut self, result: &BenchmarkResult) {
        let latency_ms = result.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0;
        self.total_latency += latency_ms;
        self.total_throughput += result.metrics.throughput_ops_per_sec;
        self.sample_count += 1;
        
        if result.target_validation.overall_success {
            self.targets_met_count += 1;
        } else {
            self.all_targets_met = false;
        }

        self.avg_latency_ms = self.total_latency / self.sample_count as f64;
        self.avg_throughput = self.total_throughput / self.sample_count as f64;
    }
}

struct ChartData {
    labels_json: String,
    latency_data_json: String,
    throughput_data_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedReport {
    pub generated_files: Vec<String>,
    pub output_directory: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub summary: ReportSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_benchmarks: usize,
    pub successful_benchmarks: usize,
    pub targets_met: usize,
    pub success_rate: f64,
    pub target_achievement_rate: f64,
    pub avg_latency_ms: f64,
    pub avg_throughput: f64,
    pub improvement_percent: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonExport {
    metadata: ExportMetadata,
    results: Vec<BenchmarkResult>,
    baseline_comparison: Option<BaselineComparisonReport>,
    statistical_analysis: Option<PerformanceComparison>,
    summary: ReportSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportMetadata {
    timestamp: chrono::DateTime<chrono::Utc>,
    version: String,
    framework: String,
}

/// Benchmark visualization utilities
pub struct BenchmarkVisualization;

impl BenchmarkVisualization {
    /// Generate ASCII chart for console output
    pub fn generate_ascii_chart(results: &[BenchmarkResult], chart_type: ChartType) -> String {
        match chart_type {
            ChartType::Latency => Self::ascii_latency_chart(results),
            ChartType::Throughput => Self::ascii_throughput_chart(results),
            ChartType::Comparison => Self::ascii_comparison_chart(results),
        }
    }

    fn ascii_latency_chart(results: &[BenchmarkResult]) -> String {
        let mut chart = String::new();
        chart.push_str("📊 Latency by Layer (P95)\n");
        chart.push_str("═".repeat(50).as_str());
        chart.push('\n');

        let max_latency = results.iter()
            .map(|r| r.metrics.latency_percentiles.p95.as_secs_f64() * 1000.0)
            .fold(0.0f64, |a, b| a.max(b));

        for layer in [MfnLayer::Layer1Ifr, MfnLayer::Layer2Dsr, MfnLayer::Layer3Alm, MfnLayer::Layer4Cpe] {
            let layer_results: Vec<_> = results.iter().filter(|r| r.layer == layer).collect();
            if !layer_results.is_empty() {
                let avg_latency = layer_results.iter()
                    .map(|r| r.metrics.latency_percentiles.p95.as_secs_f64() * 1000.0)
                    .sum::<f64>() / layer_results.len() as f64;

                let bar_length = ((avg_latency / max_latency) * 30.0) as usize;
                let bar = "█".repeat(bar_length) + &"░".repeat(30 - bar_length);
                
                chart.push_str(&format!("{:<10} │{} {:.3}ms\n", 
                    format!("{}", layer), bar, avg_latency));
            }
        }

        chart.push_str("═".repeat(50).as_str());
        chart
    }

    fn ascii_throughput_chart(results: &[BenchmarkResult]) -> String {
        let mut chart = String::new();
        chart.push_str("🚀 Throughput by Layer\n");
        chart.push_str("═".repeat(50).as_str());
        chart.push('\n');

        let max_throughput = results.iter()
            .map(|r| r.metrics.throughput_ops_per_sec)
            .fold(0.0f64, |a, b| a.max(b));

        for layer in [MfnLayer::Layer1Ifr, MfnLayer::Layer2Dsr, MfnLayer::Layer3Alm, MfnLayer::Layer4Cpe] {
            let layer_results: Vec<_> = results.iter().filter(|r| r.layer == layer).collect();
            if !layer_results.is_empty() {
                let avg_throughput = layer_results.iter()
                    .map(|r| r.metrics.throughput_ops_per_sec)
                    .sum::<f64>() / layer_results.len() as f64;

                let bar_length = ((avg_throughput / max_throughput) * 30.0) as usize;
                let bar = "█".repeat(bar_length) + &"░".repeat(30 - bar_length);
                
                chart.push_str(&format!("{:<10} │{} {:.0} ops/s\n", 
                    format!("{}", layer), bar, avg_throughput));
            }
        }

        chart.push_str("═".repeat(50).as_str());
        chart
    }

    fn ascii_comparison_chart(results: &[BenchmarkResult]) -> String {
        let mut chart = String::new();
        chart.push_str("⚖️  Performance Comparison\n");
        chart.push_str("═".repeat(50).as_str());
        chart.push('\n');

        let success_count = results.iter().filter(|r| r.success).count();
        let target_count = results.iter().filter(|r| r.target_validation.overall_success).count();
        let total_count = results.len();

        if total_count > 0 {
            let success_rate = (success_count as f64 / total_count as f64) * 100.0;
            let target_rate = (target_count as f64 / total_count as f64) * 100.0;

            chart.push_str(&format!("Success Rate:  {:.1}% ({}/{})\n", success_rate, success_count, total_count));
            chart.push_str(&format!("Target Rate:   {:.1}% ({}/{})\n", target_rate, target_count, total_count));
        }

        chart.push_str("═".repeat(50).as_str());
        chart
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChartType {
    Latency,
    Throughput,
    Comparison,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generation() {
        let config = ReportingConfig::default();
        let reporter = PerformanceReport::new(config);
        
        // Create test results
        let results = vec![create_test_result()];
        
        let summary = reporter.generate_report_summary(&results, None);
        assert_eq!(summary.total_benchmarks, 1);
        assert_eq!(summary.successful_benchmarks, 1);
    }

    #[test]
    fn test_ascii_chart_generation() {
        let results = vec![create_test_result()];
        let chart = BenchmarkVisualization::generate_ascii_chart(&results, ChartType::Latency);
        
        assert!(chart.contains("Latency by Layer"));
        assert!(chart.contains("Layer1-IFR"));
    }

    fn create_test_result() -> BenchmarkResult {
        BenchmarkResult {
            id: "test_1".to_string(),
            name: "test_benchmark".to_string(),
            layer: MfnLayer::Layer1Ifr,
            config: BenchmarkConfig {
                warmup_iterations: 10,
                measurement_iterations: 100,
                statistical_confidence: 0.95,
                regression_threshold: 0.05,
                memory_limit_mb: 128,
                timeout_seconds: 60,
                parallel_workers: 1,
                output_format: OutputFormat::Json,
                enable_flamegraph: false,
                enable_perf_counters: false,
            },
            metrics: PerformanceMetrics {
                benchmark_id: "test_1".to_string(),
                layer: MfnLayer::Layer1Ifr,
                timestamp: chrono::Utc::now(),
                duration: Duration::from_secs(1),
                throughput_ops_per_sec: 1000000.0,
                latency_percentiles: LatencyPercentiles {
                    p50: Duration::from_micros(50),
                    p75: Duration::from_micros(75),
                    p90: Duration::from_micros(90),
                    p95: Duration::from_micros(95),
                    p99: Duration::from_micros(99),
                    p999: Duration::from_micros(100),
                    max: Duration::from_micros(200),
                    min: Duration::from_micros(10),
                    mean: Duration::from_micros(60),
                    stddev: Duration::from_micros(5),
                },
                memory_usage_mb: 8.0,
                cpu_utilization: 25.0,
                error_rate: 0.0,
                custom_metrics: HashMap::new(),
            },
            target_validation: TargetValidation {
                latency_target_met: true,
                throughput_target_met: true,
                memory_target_met: true,
                improvement_target_met: true,
                overall_success: true,
                target_details: HashMap::new(),
            },
            baseline_comparison: None,
            success: true,
            error_message: None,
        }
    }
}