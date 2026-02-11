// Baseline Learning - Establishes normal behavior patterns
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use super::Metric;

/// Learns and maintains baseline behavior for metrics
pub struct BaselineLearner {
    learning_period_hours: u64,
    baselines: HashMap<String, MetricBaseline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBaseline {
    pub metric_key: String,
    pub data_points: VecDeque<DataPoint>,
    pub stats: BaselineStats,
    pub seasonal_patterns: SeasonalPatterns,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineStats {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub percentile_95: f64,
    pub percentile_99: f64,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPatterns {
    pub hourly_patterns: HashMap<u32, f64>,    // Hour of day -> avg value
    pub daily_patterns: HashMap<u32, f64>,     // Day of week -> avg value
    pub weekly_trend: f64,
}

impl BaselineLearner {
    pub fn new(learning_period_hours: u64) -> Self {
        Self {
            learning_period_hours,
            baselines: HashMap::new(),
        }
    }

    /// Update baseline with new metric
    pub fn update(&mut self, metric: &Metric) -> Result<()> {
        let key = self.get_metric_key(metric);
        let cutoff = Utc::now() - Duration::hours(self.learning_period_hours as i64);

        // Get or create baseline
        if !self.baselines.contains_key(&key) {
            self.baselines.insert(
                key.clone(),
                MetricBaseline {
                    metric_key: key.clone(),
                    data_points: VecDeque::new(),
                    stats: BaselineStats::default(),
                    seasonal_patterns: SeasonalPatterns::default(),
                    last_updated: Utc::now(),
                },
            );
        }

        // Now we can safely get mutable reference
        let baseline = self.baselines.get_mut(&key).unwrap();

        // Add data point
        baseline.data_points.push_back(DataPoint {
            timestamp: metric.timestamp,
            value: metric.value,
        });

        // Remove old data points
        while let Some(front) = baseline.data_points.front() {
            if front.timestamp < cutoff {
                baseline.data_points.pop_front();
            } else {
                break;
            }
        }

        // Clone data points for calculations
        let data_points = baseline.data_points.clone();

        // Recalculate statistics
        baseline.stats = Self::calculate_stats_static(&data_points);
        baseline.seasonal_patterns = Self::calculate_seasonal_patterns_static(&data_points);
        baseline.last_updated = Utc::now();

        Ok(())
    }

    /// Get baseline for a metric
    pub fn get_baseline(&self, metric: &Metric) -> Option<&MetricBaseline> {
        let key = self.get_metric_key(metric);
        self.baselines.get(&key)
    }

    /// Get all baseline statistics
    pub fn get_stats(&self) -> HashMap<String, BaselineStats> {
        self.baselines
            .iter()
            .map(|(k, v)| (k.clone(), v.stats.clone()))
            .collect()
    }

    fn get_metric_key(&self, metric: &Metric) -> String {
        format!(
            "{}:{}:{}:{:?}",
            metric.namespace, metric.service, metric.port, metric.metric_type
        )
    }

    fn calculate_stats_static(data_points: &VecDeque<DataPoint>) -> BaselineStats {
        if data_points.is_empty() {
            return BaselineStats::default();
        }

        let values: Vec<f64> = data_points.iter().map(|dp| dp.value).collect();
        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = values.len();
        let sum: f64 = values.iter().sum();
        let mean = sum / n as f64;

        let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();

        let median = if n % 2 == 0 {
            (sorted_values[n / 2 - 1] + sorted_values[n / 2]) / 2.0
        } else {
            sorted_values[n / 2]
        };

        let percentile_95 = sorted_values[(n as f64 * 0.95) as usize];
        let percentile_99 = sorted_values[(n as f64 * 0.99) as usize];

        BaselineStats {
            mean,
            median,
            std_dev,
            min: *sorted_values.first().unwrap(),
            max: *sorted_values.last().unwrap(),
            percentile_95,
            percentile_99,
            sample_count: n,
        }
    }

    fn calculate_seasonal_patterns_static(data_points: &VecDeque<DataPoint>) -> SeasonalPatterns {
        let mut hourly_patterns: HashMap<u32, Vec<f64>> = HashMap::new();
        let mut daily_patterns: HashMap<u32, Vec<f64>> = HashMap::new();

        for dp in data_points {
            let hour = dp.timestamp.hour();
            let day = dp.timestamp.weekday().num_days_from_monday();

            hourly_patterns
                .entry(hour)
                .or_insert_with(Vec::new)
                .push(dp.value);

            daily_patterns
                .entry(day)
                .or_insert_with(Vec::new)
                .push(dp.value);
        }

        let hourly_avgs = hourly_patterns
            .into_iter()
            .map(|(h, values)| (h, values.iter().sum::<f64>() / values.len() as f64))
            .collect();

        let daily_avgs = daily_patterns
            .into_iter()
            .map(|(d, values)| (d, values.iter().sum::<f64>() / values.len() as f64))
            .collect();

        // Calculate weekly trend (simple linear regression)
        let weekly_trend = if data_points.len() > 2 {
            let first_half: f64 = data_points
                .iter()
                .take(data_points.len() / 2)
                .map(|dp| dp.value)
                .sum::<f64>()
                / (data_points.len() / 2) as f64;

            let second_half: f64 = data_points
                .iter()
                .skip(data_points.len() / 2)
                .map(|dp| dp.value)
                .sum::<f64>()
                / (data_points.len() - data_points.len() / 2) as f64;

            (second_half - first_half) / first_half
        } else {
            0.0
        };

        SeasonalPatterns {
            hourly_patterns: hourly_avgs,
            daily_patterns: daily_avgs,
            weekly_trend,
        }
    }
}

impl Default for BaselineStats {
    fn default() -> Self {
        Self {
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
            min: 0.0,
            max: 0.0,
            percentile_95: 0.0,
            percentile_99: 0.0,
            sample_count: 0,
        }
    }
}

impl Default for SeasonalPatterns {
    fn default() -> Self {
        Self {
            hourly_patterns: HashMap::new(),
            daily_patterns: HashMap::new(),
            weekly_trend: 0.0,
        }
    }
}
