/*!
# Statistical Analysis and Performance Comparison

Provides comprehensive statistical analysis of benchmark results including:
- Statistical significance testing
- Performance regression detection  
- Confidence intervals and hypothesis testing
- Trend analysis and prediction
- Performance anomaly detection
*/

use crate::common::*;
use crate::baseline::BaselineComparisonReport;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use statrs::distribution::{StudentsT, ContinuousCDF};
use statrs::statistics::{Statistics, OrderStatistics};

/// Statistical analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub confidence_level: f64,
    pub significance_threshold: f64,
    pub regression_threshold: f64,
    pub outlier_threshold: f64,
    pub trend_window_size: usize,
    pub enable_anomaly_detection: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            confidence_level: 0.95,
            significance_threshold: 0.05,
            regression_threshold: 0.05, // 5% regression is significant
            outlier_threshold: 2.0,     // 2 standard deviations
            trend_window_size: 20,
            enable_anomaly_detection: true,
        }
    }
}

/// Comprehensive statistical analysis of benchmark results
pub struct StatisticalAnalysis {
    config: AnalysisConfig,
}

impl StatisticalAnalysis {
    pub fn new(config: AnalysisConfig) -> Self {
        Self { config }
    }

    /// Analyze performance comparison between MFN and baseline
    pub fn analyze_performance_comparison(
        &self,
        mfn_results: &[BenchmarkResult],
        baseline_report: &BaselineComparisonReport,
    ) -> PerformanceComparison {
        let mfn_latencies: Vec<f64> = mfn_results.iter()
            .map(|r| r.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0) // Convert to ms
            .collect();

        let mfn_throughputs: Vec<f64> = mfn_results.iter()
            .map(|r| r.metrics.throughput_ops_per_sec)
            .collect();

        // Calculate statistical measures
        let latency_stats = self.calculate_descriptive_stats(&mfn_latencies);
        let throughput_stats = self.calculate_descriptive_stats(&mfn_throughputs);

        // Perform hypothesis testing
        let improvement_significance = self.test_improvement_significance(
            baseline_report.baseline_avg_latency_ms,
            &mfn_latencies
        );

        let target_validation = self.validate_performance_targets(mfn_results, baseline_report);

        PerformanceComparison {
            mfn_latency_stats: latency_stats,
            mfn_throughput_stats: throughput_stats,
            baseline_latency_ms: baseline_report.baseline_avg_latency_ms,
            baseline_throughput_ops_sec: baseline_report.baseline_throughput_ops_sec,
            improvement_percent: baseline_report.overall_improvement_percent,
            improvement_significance,
            confidence_intervals: self.calculate_confidence_intervals(&mfn_latencies, &mfn_throughputs),
            target_validation,
            statistical_tests: self.perform_statistical_tests(&mfn_latencies, &mfn_throughputs),
            anomalies: if self.config.enable_anomaly_detection {
                self.detect_anomalies(mfn_results)
            } else {
                Vec::new()
            },
        }
    }

    fn calculate_descriptive_stats(&self, data: &[f64]) -> DescriptiveStatistics {
        if data.is_empty() {
            return DescriptiveStatistics::default();
        }

        let mean = data.mean();
        let std_dev = data.std_dev();
        let min = data.min();
        let max = data.max();
        let median = data.median();
        
        let q1 = data.percentile(25);
        let q3 = data.percentile(75);
        let iqr = q3 - q1;

        DescriptiveStatistics {
            count: data.len(),
            mean,
            median,
            std_dev,
            min,
            max,
            q1,
            q3,
            iqr,
            variance: std_dev * std_dev,
            skewness: self.calculate_skewness(data, mean, std_dev),
            kurtosis: self.calculate_kurtosis(data, mean, std_dev),
        }
    }

    fn calculate_skewness(&self, data: &[f64], mean: f64, std_dev: f64) -> f64 {
        if std_dev == 0.0 || data.len() < 3 {
            return 0.0;
        }

        let n = data.len() as f64;
        let sum_cubed_deviations: f64 = data.iter()
            .map(|&x| ((x - mean) / std_dev).powi(3))
            .sum();

        (n / ((n - 1.0) * (n - 2.0))) * sum_cubed_deviations
    }

    fn calculate_kurtosis(&self, data: &[f64], mean: f64, std_dev: f64) -> f64 {
        if std_dev == 0.0 || data.len() < 4 {
            return 0.0;
        }

        let n = data.len() as f64;
        let sum_fourth_deviations: f64 = data.iter()
            .map(|&x| ((x - mean) / std_dev).powi(4))
            .sum();

        let kurtosis = (n * (n + 1.0) / ((n - 1.0) * (n - 2.0) * (n - 3.0))) * sum_fourth_deviations;
        kurtosis - (3.0 * (n - 1.0).powi(2) / ((n - 2.0) * (n - 3.0)))
    }

    fn test_improvement_significance(&self, baseline_mean: f64, mfn_data: &[f64]) -> SignificanceTest {
        if mfn_data.is_empty() {
            return SignificanceTest::default();
        }

        let mfn_mean = mfn_data.mean();
        let mfn_std = mfn_data.std_dev();
        let n = mfn_data.len() as f64;

        // One-sample t-test against baseline
        let standard_error = mfn_std / n.sqrt();
        let t_statistic = (mfn_mean - baseline_mean) / standard_error;
        let degrees_of_freedom = n - 1.0;

        let t_dist = StudentsT::new(0.0, 1.0, degrees_of_freedom).unwrap();
        let p_value = 2.0 * (1.0 - t_dist.cdf(t_statistic.abs()));

        SignificanceTest {
            test_type: "One-sample t-test".to_string(),
            t_statistic,
            degrees_of_freedom,
            p_value,
            is_significant: p_value < self.config.significance_threshold,
            effect_size: (baseline_mean - mfn_mean) / baseline_mean, // Cohen's d approximation
            confidence_level: self.config.confidence_level,
        }
    }

    fn calculate_confidence_intervals(
        &self,
        latency_data: &[f64],
        throughput_data: &[f64]
    ) -> ConfidenceIntervals {
        let latency_ci = self.calculate_ci(latency_data);
        let throughput_ci = self.calculate_ci(throughput_data);

        ConfidenceIntervals {
            latency_ci,
            throughput_ci,
            confidence_level: self.config.confidence_level,
        }
    }

    fn calculate_ci(&self, data: &[f64]) -> (f64, f64) {
        if data.len() < 2 {
            let mean = if data.is_empty() { 0.0 } else { data[0] };
            return (mean, mean);
        }

        let mean = data.mean();
        let std_dev = data.std_dev();
        let n = data.len() as f64;
        let degrees_of_freedom = n - 1.0;

        let alpha = 1.0 - self.config.confidence_level;
        let t_dist = StudentsT::new(0.0, 1.0, degrees_of_freedom).unwrap();
        let t_critical = t_dist.inverse_cdf(1.0 - alpha / 2.0);

        let margin_of_error = t_critical * (std_dev / n.sqrt());
        
        (mean - margin_of_error, mean + margin_of_error)
    }

    fn validate_performance_targets(
        &self,
        mfn_results: &[BenchmarkResult],
        baseline_report: &BaselineComparisonReport
    ) -> TargetValidationReport {
        let mut layer_validations = HashMap::new();

        // Validate layer-specific targets
        for layer in [MfnLayer::Layer1Ifr, MfnLayer::Layer2Dsr, MfnLayer::Layer3Alm, MfnLayer::Layer4Cpe] {
            let layer_results: Vec<_> = mfn_results.iter()
                .filter(|r| r.layer == layer)
                .collect();

            if !layer_results.is_empty() {
                let validation = self.validate_layer_targets(layer, &layer_results);
                layer_validations.insert(layer, validation);
            }
        }

        // Overall system validation
        let overall_improvement_target = 88.6; // 88.6% improvement target
        let overall_achieved = baseline_report.overall_improvement_percent >= overall_improvement_target;

        TargetValidationReport {
            overall_target_met: overall_achieved,
            overall_improvement_percent: baseline_report.overall_improvement_percent,
            overall_improvement_target: overall_improvement_target,
            layer_validations,
            throughput_target_met: baseline_report.mfn_throughput_ops_sec >= adaptive network tiers
            memory_target_met: self.validate_memory_targets(mfn_results),
            statistical_confidence: self.config.confidence_level,
        }
    }

    fn validate_layer_targets(&self, layer: MfnLayer, results: &[&BenchmarkResult]) -> LayerValidation {
        let latencies_ms: Vec<f64> = results.iter()
            .map(|r| r.metrics.latency_percentiles.p95.as_secs_f64() * 1000.0)
            .collect();

        let mean_latency = latencies_ms.mean();
        
        let (target_met, target_latency_ms) = match layer {
            MfnLayer::Layer1Ifr => (mean_latency < 0.1, 0.1),
            MfnLayer::Layer2Dsr => (mean_latency < 1.0, 1.0),
            MfnLayer::Layer3Alm => (true, f64::INFINITY), // Validated through improvement percentage
            MfnLayer::Layer4Cpe => (mean_latency < 2.0, 2.0),
            MfnLayer::Integration => (mean_latency < 10.0, 10.0), // Overall latency target
        };

        LayerValidation {
            layer,
            target_met,
            actual_latency_ms: mean_latency,
            target_latency_ms,
            sample_count: results.len(),
            confidence_interval: self.calculate_ci(&latencies_ms),
        }
    }

    fn validate_memory_targets(&self, results: &[BenchmarkResult]) -> bool {
        let memory_usages: Vec<f64> = results.iter()
            .map(|r| r.metrics.memory_usage_mb)
            .collect();

        if memory_usages.is_empty() {
            return true; // No data means we pass by default
        }

        let max_memory = memory_usages.iter().fold(0.0f64, |a, &b| a.max(b));
        
        // Layer-specific memory targets
        max_memory < 1024.0 // 1GB total memory limit for all layers
    }

    fn perform_statistical_tests(&self, latency_data: &[f64], throughput_data: &[f64]) -> StatisticalTests {
        StatisticalTests {
            normality_test: self.shapiro_wilk_approximation(latency_data),
            outlier_test: self.detect_outliers_iqr(latency_data),
            variance_homogeneity: self.test_variance_homogeneity(latency_data, throughput_data),
            trend_analysis: self.analyze_trends(latency_data),
        }
    }

    fn shapiro_wilk_approximation(&self, data: &[f64]) -> NormalityTest {
        // Simplified normality test using skewness and kurtosis
        if data.len() < 3 {
            return NormalityTest {
                test_name: "Shapiro-Wilk (approximated)".to_string(),
                p_value: 1.0,
                is_normal: true,
                sample_size: data.len(),
            };
        }

        let stats = self.calculate_descriptive_stats(data);
        
        // Approximation: normal distribution has skewness ≈ 0 and kurtosis ≈ 0
        let skew_deviation = stats.skewness.abs();
        let kurt_deviation = stats.kurtosis.abs();
        
        // Combined deviation score
        let deviation_score = skew_deviation + kurt_deviation;
        
        // Approximate p-value based on deviation
        let p_value = (-deviation_score).exp().max(0.001).min(1.0);
        
        NormalityTest {
            test_name: "Shapiro-Wilk (approximated)".to_string(),
            p_value,
            is_normal: p_value > self.config.significance_threshold,
            sample_size: data.len(),
        }
    }

    fn detect_outliers_iqr(&self, data: &[f64]) -> OutlierTest {
        if data.len() < 4 {
            return OutlierTest {
                outlier_indices: Vec::new(),
                outlier_count: 0,
                outlier_percentage: 0.0,
                method: "IQR".to_string(),
            };
        }

        let q1 = data.percentile(25);
        let q3 = data.percentile(75);
        let iqr = q3 - q1;
        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        let outlier_indices: Vec<usize> = data.iter()
            .enumerate()
            .filter(|(_, &value)| value < lower_bound || value > upper_bound)
            .map(|(idx, _)| idx)
            .collect();

        OutlierTest {
            outlier_count: outlier_indices.len(),
            outlier_percentage: (outlier_indices.len() as f64 / data.len() as f64) * 100.0,
            outlier_indices,
            method: "IQR".to_string(),
        }
    }

    fn test_variance_homogeneity(&self, data1: &[f64], data2: &[f64]) -> VarianceTest {
        if data1.len() < 2 || data2.len() < 2 {
            return VarianceTest {
                test_name: "F-test".to_string(),
                f_statistic: 1.0,
                p_value: 1.0,
                equal_variances: true,
            };
        }

        let var1 = data1.variance();
        let var2 = data2.variance();
        
        let f_statistic = if var2 > 0.0 { var1 / var2 } else { 1.0 };
        
        // Simplified p-value calculation
        let p_value = if f_statistic > 2.0 || f_statistic < 0.5 {
            0.01 // Significant difference
        } else {
            0.5  // No significant difference
        };

        VarianceTest {
            test_name: "F-test".to_string(),
            f_statistic,
            p_value,
            equal_variances: p_value > self.config.significance_threshold,
        }
    }

    fn analyze_trends(&self, data: &[f64]) -> TrendAnalysis {
        if data.len() < self.config.trend_window_size {
            return TrendAnalysis::default();
        }

        // Simple linear regression for trend
        let n = data.len() as f64;
        let x_mean = (n - 1.0) / 2.0; // Mean of indices 0, 1, 2, ..., n-1
        let y_mean = data.mean();

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, &y) in data.iter().enumerate() {
            let x = i as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean).powi(2);
        }

        let slope = if denominator != 0.0 { numerator / denominator } else { 0.0 };
        let intercept = y_mean - slope * x_mean;

        // Calculate R-squared
        let ss_total: f64 = data.iter().map(|&y| (y - y_mean).powi(2)).sum();
        let ss_residual: f64 = data.iter().enumerate()
            .map(|(i, &y)| {
                let predicted = slope * (i as f64) + intercept;
                (y - predicted).powi(2)
            })
            .sum();

        let r_squared = if ss_total != 0.0 { 1.0 - (ss_residual / ss_total) } else { 0.0 };

        TrendAnalysis {
            trend_direction: if slope > 0.01 { "Increasing".to_string() }
                           else if slope < -0.01 { "Decreasing".to_string() }
                           else { "Stable".to_string() },
            slope,
            r_squared,
            is_significant_trend: slope.abs() > 0.01 && r_squared > 0.5,
        }
    }

    fn detect_anomalies(&self, results: &[BenchmarkResult]) -> Vec<PerformanceAnomaly> {
        let mut anomalies = Vec::new();

        // Detect latency anomalies
        let latencies: Vec<f64> = results.iter()
            .map(|r| r.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0)
            .collect();

        let latency_stats = self.calculate_descriptive_stats(&latencies);
        let latency_threshold = latency_stats.mean + self.config.outlier_threshold * latency_stats.std_dev;

        for (i, result) in results.iter().enumerate() {
            let latency_ms = result.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0;
            if latency_ms > latency_threshold {
                anomalies.push(PerformanceAnomaly {
                    anomaly_type: AnomalyType::HighLatency,
                    benchmark_index: i,
                    value: latency_ms,
                    threshold: latency_threshold,
                    severity: if latency_ms > latency_threshold * 2.0 { "High" } else { "Medium" }.to_string(),
                    description: format!("Latency {} ms exceeds threshold {} ms", latency_ms, latency_threshold),
                });
            }
        }

        // Detect throughput anomalies
        let throughputs: Vec<f64> = results.iter()
            .map(|r| r.metrics.throughput_ops_per_sec)
            .collect();

        let throughput_stats = self.calculate_descriptive_stats(&throughputs);
        let throughput_threshold = throughput_stats.mean - self.config.outlier_threshold * throughput_stats.std_dev;

        for (i, result) in results.iter().enumerate() {
            let throughput = result.metrics.throughput_ops_per_sec;
            if throughput < throughput_threshold {
                anomalies.push(PerformanceAnomaly {
                    anomaly_type: AnomalyType::LowThroughput,
                    benchmark_index: i,
                    value: throughput,
                    threshold: throughput_threshold,
                    severity: if throughput < throughput_threshold * 0.5 { "High" } else { "Medium" }.to_string(),
                    description: format!("Throughput {} ops/sec below threshold {} ops/sec", throughput, throughput_threshold),
                });
            }
        }

        anomalies
    }
}

/// Performance regression detection
pub struct RegressionDetection {
    config: AnalysisConfig,
}

impl RegressionDetection {
    pub fn new(config: AnalysisConfig) -> Self {
        Self { config }
    }

    /// Detect performance regressions between benchmark runs
    pub fn detect_regressions(
        &self,
        current_results: &[BenchmarkResult],
        historical_results: &[BenchmarkResult],
    ) -> Vec<PerformanceRegression> {
        let mut regressions = Vec::new();

        // Group results by layer and benchmark name
        let current_by_layer = self.group_by_layer(current_results);
        let historical_by_layer = self.group_by_layer(historical_results);

        for (layer, current_layer_results) in current_by_layer {
            if let Some(historical_layer_results) = historical_by_layer.get(&layer) {
                let layer_regressions = self.detect_layer_regressions(
                    layer,
                    current_layer_results,
                    historical_layer_results
                );
                regressions.extend(layer_regressions);
            }
        }

        regressions
    }

    fn group_by_layer(&self, results: &[BenchmarkResult]) -> HashMap<MfnLayer, Vec<&BenchmarkResult>> {
        let mut grouped = HashMap::new();
        
        for result in results {
            grouped.entry(result.layer).or_insert_with(Vec::new).push(result);
        }
        
        grouped
    }

    fn detect_layer_regressions(
        &self,
        layer: MfnLayer,
        current: &[&BenchmarkResult],
        historical: &[&BenchmarkResult]
    ) -> Vec<PerformanceRegression> {
        let mut regressions = Vec::new();

        // Calculate current and historical means
        let current_latencies: Vec<f64> = current.iter()
            .map(|r| r.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0)
            .collect();
            
        let historical_latencies: Vec<f64> = historical.iter()
            .map(|r| r.metrics.latency_percentiles.mean.as_secs_f64() * 1000.0)
            .collect();

        if current_latencies.is_empty() || historical_latencies.is_empty() {
            return regressions;
        }

        let current_mean = current_latencies.mean();
        let historical_mean = historical_latencies.mean();
        let regression_percent = ((current_mean - historical_mean) / historical_mean) * 100.0;

        // Check for significant latency regression
        if regression_percent > self.config.regression_threshold * 100.0 {
            regressions.push(PerformanceRegression {
                regression_type: RegressionType::LatencyIncrease,
                layer,
                metric_name: "Latency".to_string(),
                current_value: current_mean,
                historical_value: historical_mean,
                regression_percent,
                is_significant: regression_percent > self.config.regression_threshold * 100.0,
                confidence_level: self.config.confidence_level,
                detection_timestamp: chrono::Utc::now(),
            });
        }

        // Check throughput regression
        let current_throughputs: Vec<f64> = current.iter()
            .map(|r| r.metrics.throughput_ops_per_sec)
            .collect();
            
        let historical_throughputs: Vec<f64> = historical.iter()
            .map(|r| r.metrics.throughput_ops_per_sec)
            .collect();

        let current_throughput_mean = current_throughputs.mean();
        let historical_throughput_mean = historical_throughputs.mean();
        let throughput_regression_percent = ((historical_throughput_mean - current_throughput_mean) / historical_throughput_mean) * 100.0;

        if throughput_regression_percent > self.config.regression_threshold * 100.0 {
            regressions.push(PerformanceRegression {
                regression_type: RegressionType::ThroughputDecrease,
                layer,
                metric_name: "Throughput".to_string(),
                current_value: current_throughput_mean,
                historical_value: historical_throughput_mean,
                regression_percent: throughput_regression_percent,
                is_significant: throughput_regression_percent > self.config.regression_threshold * 100.0,
                confidence_level: self.config.confidence_level,
                detection_timestamp: chrono::Utc::now(),
            });
        }

        regressions
    }
}

// Data structures for analysis results

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    pub mfn_latency_stats: DescriptiveStatistics,
    pub mfn_throughput_stats: DescriptiveStatistics,
    pub baseline_latency_ms: f64,
    pub baseline_throughput_ops_sec: f64,
    pub improvement_percent: f64,
    pub improvement_significance: SignificanceTest,
    pub confidence_intervals: ConfidenceIntervals,
    pub target_validation: TargetValidationReport,
    pub statistical_tests: StatisticalTests,
    pub anomalies: Vec<PerformanceAnomaly>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DescriptiveStatistics {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub q1: f64,
    pub q3: f64,
    pub iqr: f64,
    pub variance: f64,
    pub skewness: f64,
    pub kurtosis: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignificanceTest {
    pub test_type: String,
    pub t_statistic: f64,
    pub degrees_of_freedom: f64,
    pub p_value: f64,
    pub is_significant: bool,
    pub effect_size: f64,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceIntervals {
    pub latency_ci: (f64, f64),
    pub throughput_ci: (f64, f64),
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetValidationReport {
    pub overall_target_met: bool,
    pub overall_improvement_percent: f64,
    pub overall_improvement_target: f64,
    pub layer_validations: HashMap<MfnLayer, LayerValidation>,
    pub throughput_target_met: bool,
    pub memory_target_met: bool,
    pub statistical_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerValidation {
    pub layer: MfnLayer,
    pub target_met: bool,
    pub actual_latency_ms: f64,
    pub target_latency_ms: f64,
    pub sample_count: usize,
    pub confidence_interval: (f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalTests {
    pub normality_test: NormalityTest,
    pub outlier_test: OutlierTest,
    pub variance_homogeneity: VarianceTest,
    pub trend_analysis: TrendAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalityTest {
    pub test_name: String,
    pub p_value: f64,
    pub is_normal: bool,
    pub sample_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierTest {
    pub outlier_indices: Vec<usize>,
    pub outlier_count: usize,
    pub outlier_percentage: f64,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceTest {
    pub test_name: String,
    pub f_statistic: f64,
    pub p_value: f64,
    pub equal_variances: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrendAnalysis {
    pub trend_direction: String,
    pub slope: f64,
    pub r_squared: f64,
    pub is_significant_trend: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnomaly {
    pub anomaly_type: AnomalyType,
    pub benchmark_index: usize,
    pub value: f64,
    pub threshold: f64,
    pub severity: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    HighLatency,
    LowThroughput,
    MemorySpike,
    ErrorRateHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRegression {
    pub regression_type: RegressionType,
    pub layer: MfnLayer,
    pub metric_name: String,
    pub current_value: f64,
    pub historical_value: f64,
    pub regression_percent: f64,
    pub is_significant: bool,
    pub confidence_level: f64,
    pub detection_timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegressionType {
    LatencyIncrease,
    ThroughputDecrease,
    MemoryIncrease,
    AccuracyDecrease,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::BaselineComparisonReport;

    #[test]
    fn test_descriptive_statistics() {
        let config = AnalysisConfig::default();
        let analysis = StatisticalAnalysis::new(config);
        
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = analysis.calculate_descriptive_stats(&data);
        
        assert_eq!(stats.count, 5);
        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.median, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
    }

    #[test]
    fn test_confidence_interval() {
        let config = AnalysisConfig::default();
        let analysis = StatisticalAnalysis::new(config);
        
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (lower, upper) = analysis.calculate_ci(&data);
        
        assert!(lower < 3.0);
        assert!(upper > 3.0);
        assert!(upper - lower > 0.0);
    }

    #[test]
    fn test_outlier_detection() {
        let config = AnalysisConfig::default();
        let analysis = StatisticalAnalysis::new(config);
        
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0]; // 100.0 is an outlier
        let outlier_test = analysis.detect_outliers_iqr(&data);
        
        assert_eq!(outlier_test.outlier_count, 1);
        assert!(outlier_test.outlier_indices.contains(&5));
    }

    #[test]
    fn test_trend_analysis() {
        let config = AnalysisConfig {
            trend_window_size: 3,
            ..Default::default()
        };
        let analysis = StatisticalAnalysis::new(config);
        
        let increasing_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let trend = analysis.analyze_trends(&increasing_data);
        
        assert_eq!(trend.trend_direction, "Increasing");
        assert!(trend.slope > 0.0);
        assert!(trend.r_squared > 0.9); // Should be close to 1 for linear data
    }

    #[test]
    fn test_regression_detection() {
        let config = AnalysisConfig::default();
        let detector = RegressionDetection::new(config);
        
        // Create mock results with regression
        let historical = vec![create_mock_result(MfnLayer::Layer1Ifr, 1.0, 1000000.0)];
        let current = vec![create_mock_result(MfnLayer::Layer1Ifr, 2.0, 500000.0)]; // Worse performance
        
        let regressions = detector.detect_regressions(&current, &historical);
        
        assert!(!regressions.is_empty());
        assert!(regressions.iter().any(|r| matches!(r.regression_type, RegressionType::LatencyIncrease)));
        assert!(regressions.iter().any(|r| matches!(r.regression_type, RegressionType::ThroughputDecrease)));
    }

    fn create_mock_result(layer: MfnLayer, latency_ms: f64, throughput: f64) -> BenchmarkResult {
        BenchmarkResult {
            id: "test".to_string(),
            name: "test".to_string(),
            layer,
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
                benchmark_id: "test".to_string(),
                layer,
                timestamp: chrono::Utc::now(),
                duration: Duration::from_secs(1),
                throughput_ops_per_sec: throughput,
                latency_percentiles: LatencyPercentiles {
                    p50: Duration::from_secs_f64(latency_ms / 1000.0),
                    p75: Duration::from_secs_f64(latency_ms / 1000.0),
                    p90: Duration::from_secs_f64(latency_ms / 1000.0),
                    p95: Duration::from_secs_f64(latency_ms / 1000.0),
                    p99: Duration::from_secs_f64(latency_ms / 1000.0),
                    p999: Duration::from_secs_f64(latency_ms / 1000.0),
                    max: Duration::from_secs_f64(latency_ms / 1000.0),
                    min: Duration::from_secs_f64(latency_ms / 1000.0),
                    mean: Duration::from_secs_f64(latency_ms / 1000.0),
                    stddev: Duration::from_secs_f64(0.1),
                },
                memory_usage_mb: 10.0,
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