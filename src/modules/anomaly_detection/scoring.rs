// Anomaly Scoring - Multiple ML algorithms for anomaly detection
use anyhow::Result;
use chrono::Timelike;
use serde::{Deserialize, Serialize};

use super::{Algorithm, AnomalyType, Metric};
use super::baseline::{BaselineLearner, BaselineStats};

/// Multi-algorithm anomaly scorer
pub struct AnomalyScorer {
    sensitivity: f64,
    algorithms: Vec<Algorithm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyScore {
    pub confidence: f64,
    pub deviation: f64,
    pub anomaly_type: AnomalyType,
    pub baseline_value: f64,
    pub algorithm_scores: Vec<AlgorithmScore>,
    pub contributing_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmScore {
    pub algorithm: Algorithm,
    pub score: f64,
    pub triggered: bool,
}

impl AnomalyScorer {
    pub fn new(sensitivity: f64, algorithms: Vec<Algorithm>) -> Self {
        Self {
            sensitivity,
            algorithms,
        }
    }

    /// Score a metric against its baseline using multiple algorithms
    pub fn score(
        &self,
        metric: &Metric,
        baseline_learner: &BaselineLearner,
    ) -> Result<Option<AnomalyScore>> {
        let baseline = match baseline_learner.get_baseline(metric) {
            Some(b) => b,
            None => return Ok(None), // No baseline yet
        };

        // Need minimum samples for reliable detection
        if baseline.stats.sample_count < 100 {
            return Ok(None);
        }

        let mut algorithm_scores = Vec::new();
        let mut total_score = 0.0;
        let mut triggered_count = 0;

        // Run each algorithm
        for algo in &self.algorithms {
            let score = match algo {
                Algorithm::ZScore => self.z_score(metric, &baseline.stats)?,
                Algorithm::IsolationForest => self.isolation_forest(metric, baseline)?,
                Algorithm::LSTM => self.lstm_score(metric, baseline)?,
                Algorithm::MACD => self.macd_score(metric, baseline)?,
                Algorithm::SeasonalHybridESD => self.seasonal_esd(metric, baseline)?,
            };

            let triggered = score > self.get_threshold(algo);
            if triggered {
                triggered_count += 1;
            }

            total_score += score;
            algorithm_scores.push(AlgorithmScore {
                algorithm: algo.clone(),
                score,
                triggered,
            });
        }

        // Need at least 2 algorithms to agree for high confidence
        if triggered_count < 2 {
            return Ok(None);
        }

        let avg_score = total_score / self.algorithms.len() as f64;
        // Confidence is based on algorithm agreement ratio, weighted by average score.
        // avg_score is already in 0.0..1.0 range, so multiplying by agreement ratio
        // gives a proper confidence value without double-normalization.
        let agreement_ratio = triggered_count as f64 / self.algorithms.len() as f64;
        let confidence = (agreement_ratio * avg_score).clamp(0.0, 1.0);

        // Calculate deviation from baseline
        let deviation = ((metric.value - baseline.stats.mean) / baseline.stats.std_dev).abs();

        // Determine anomaly type
        let anomaly_type = self.classify_anomaly(metric, &baseline.stats, deviation);

        // Identify contributing factors
        let contributing_factors = self.identify_factors(metric, baseline, deviation);

        Ok(Some(AnomalyScore {
            confidence,
            deviation,
            anomaly_type,
            baseline_value: baseline.stats.mean,
            algorithm_scores,
            contributing_factors,
        }))
    }

    /// Z-Score algorithm: Statistical deviation from mean.
    /// Calculates the number of standard deviations the metric value is from the
    /// baseline mean. Values beyond the sensitivity-adjusted threshold (default ~3
    /// stddevs) are scored proportionally, clamped to 0.0..1.0.
    fn z_score(&self, metric: &Metric, stats: &BaselineStats) -> Result<f64> {
        if stats.std_dev == 0.0 {
            // No variance in baseline - any deviation from mean is anomalous
            if (metric.value - stats.mean).abs() > f64::EPSILON {
                return Ok(1.0);
            }
            return Ok(0.0);
        }

        let z = ((metric.value - stats.mean) / stats.std_dev).abs();

        // Threshold scales with sensitivity: high sensitivity lowers the bar.
        // At sensitivity 0.0 -> threshold 3.0, at 1.0 -> threshold 2.0
        let threshold = 3.0 - self.sensitivity;

        // Score proportionally: z at threshold -> 0.5, z at 2*threshold -> 1.0
        let score = if z <= threshold * 0.5 {
            0.0
        } else {
            ((z - threshold * 0.5) / (threshold * 1.5)).clamp(0.0, 1.0)
        };

        Ok(score)
    }

    /// IQR-based detection: Uses interquartile range for robust outlier detection.
    /// Less sensitive to extreme outliers than Z-Score. Flags values outside
    /// Q1 - 1.5*IQR or Q3 + 1.5*IQR as anomalous.
    fn iqr_score(&self, metric: &Metric, stats: &BaselineStats) -> Result<f64> {
        // Approximate Q1 and Q3 from available stats.
        // Q1 ~ mean - 0.675*stddev, Q3 ~ mean + 0.675*stddev (normal distribution)
        let q1 = stats.mean - 0.675 * stats.std_dev;
        let q3 = stats.mean + 0.675 * stats.std_dev;
        let iqr = q3 - q1;

        if iqr < f64::EPSILON {
            if (metric.value - stats.mean).abs() > f64::EPSILON {
                return Ok(1.0);
            }
            return Ok(0.0);
        }

        let lower_fence = q1 - 1.5 * iqr;
        let upper_fence = q3 + 1.5 * iqr;
        let extreme_lower = q1 - 3.0 * iqr;
        let extreme_upper = q3 + 3.0 * iqr;

        let value = metric.value;
        if value >= lower_fence && value <= upper_fence {
            Ok(0.0) // Within normal range
        } else if value < extreme_lower || value > extreme_upper {
            Ok(1.0) // Extreme outlier
        } else {
            // Mild outlier - score proportionally
            let distance = if value < lower_fence {
                (lower_fence - value) / (lower_fence - extreme_lower)
            } else {
                (value - upper_fence) / (extreme_upper - upper_fence)
            };
            Ok(distance.clamp(0.0, 1.0))
        }
    }

    /// Simplified Isolation Forest: Measures how "isolated" a point is
    fn isolation_forest(&self, metric: &Metric, baseline: &super::baseline::MetricBaseline) -> Result<f64> {
        // Simplified version: compare against percentiles
        let value = metric.value;
        let stats = &baseline.stats;

        if value > stats.percentile_99 || value < (stats.mean - 3.0 * stats.std_dev) {
            Ok(1.0)
        } else if value > stats.percentile_95 || value < (stats.mean - 2.0 * stats.std_dev) {
            Ok(0.7)
        } else {
            Ok(0.0)
        }
    }

    /// LSTM-inspired: Pattern matching against historical sequences
    fn lstm_score(&self, metric: &Metric, baseline: &super::baseline::MetricBaseline) -> Result<f64> {
        // Simplified: Check if recent trend matches historical patterns
        let recent_data: Vec<f64> = baseline
            .data_points
            .iter()
            .rev()
            .take(10)
            .map(|dp| dp.value)
            .collect();

        if recent_data.len() < 5 {
            return Ok(0.0);
        }

        let recent_avg = recent_data.iter().sum::<f64>() / recent_data.len() as f64;
        let deviation = ((metric.value - recent_avg) / recent_avg).abs();

        Ok((deviation * 2.0).min(1.0))
    }

    /// MACD: Moving Average Convergence Divergence
    fn macd_score(&self, metric: &Metric, baseline: &super::baseline::MetricBaseline) -> Result<f64> {
        // Calculate short and long moving averages
        let short_window = 12;
        let long_window = 26;

        if baseline.data_points.len() < long_window {
            return Ok(0.0);
        }

        let short_ma: f64 = baseline
            .data_points
            .iter()
            .rev()
            .take(short_window)
            .map(|dp| dp.value)
            .sum::<f64>()
            / short_window as f64;

        let long_ma: f64 = baseline
            .data_points
            .iter()
            .rev()
            .take(long_window)
            .map(|dp| dp.value)
            .sum::<f64>()
            / long_window as f64;

        let macd = short_ma - long_ma;
        let signal = macd / long_ma;

        // Current value deviating from MACD signal
        let current_signal = (metric.value - long_ma) / long_ma;
        let divergence = (current_signal - signal).abs();

        Ok((divergence * 5.0).min(1.0))
    }

    /// Seasonal Hybrid ESD: Accounts for seasonal patterns
    fn seasonal_esd(&self, metric: &Metric, baseline: &super::baseline::MetricBaseline) -> Result<f64> {
        let hour = metric.timestamp.hour();
        let expected = baseline
            .seasonal_patterns
            .hourly_patterns
            .get(&hour)
            .copied()
            .unwrap_or(baseline.stats.mean);

        let deviation = ((metric.value - expected) / expected).abs();

        Ok((deviation * 2.0).min(1.0))
    }

    fn get_threshold(&self, algo: &Algorithm) -> f64 {
        let base_threshold = match algo {
            Algorithm::ZScore => 0.6,
            Algorithm::IsolationForest => 0.7,
            Algorithm::LSTM => 0.5,
            Algorithm::MACD => 0.4,
            Algorithm::SeasonalHybridESD => 0.5,
        };

        base_threshold * (1.0 - self.sensitivity * 0.3)
    }

    fn classify_anomaly(&self, metric: &Metric, stats: &BaselineStats, deviation: f64) -> AnomalyType {
        use super::MetricType;

        match metric.metric_type {
            MetricType::RequestRate if metric.value > stats.mean * 2.0 => AnomalyType::TrafficSpike,
            MetricType::RequestRate if metric.value < stats.mean * 0.5 => AnomalyType::TrafficDrop,
            MetricType::ErrorRate if metric.value > stats.mean * 1.5 => AnomalyType::ErrorRateSpike,
            MetricType::Latency if metric.value > stats.mean * 1.5 => AnomalyType::LatencyIncrease,
            MetricType::ConnectionCount if deviation > 5.0 => AnomalyType::UnusualConnectionPattern,
            MetricType::UniqueDestinations if metric.value > stats.percentile_95 * 1.5 => {
                AnomalyType::PortScan
            }
            MetricType::DNSQueryRate if metric.value > stats.mean * 3.0 => AnomalyType::DNSTunneling,
            MetricType::BytesTransferred if metric.value > stats.percentile_99 * 2.0 => {
                AnomalyType::DataExfiltration
            }
            _ => AnomalyType::BehavioralAnomaly,
        }
    }

    fn identify_factors(
        &self,
        metric: &Metric,
        baseline: &super::baseline::MetricBaseline,
        deviation: f64,
    ) -> Vec<String> {
        let mut factors = Vec::new();

        if deviation > 5.0 {
            factors.push("Extreme deviation from baseline".to_string());
        }

        if metric.value > baseline.stats.percentile_99 {
            factors.push("Value exceeds 99th percentile".to_string());
        }

        if baseline.seasonal_patterns.weekly_trend.abs() > 0.2 {
            factors.push(format!(
                "Weekly trend: {:.1}%",
                baseline.seasonal_patterns.weekly_trend * 100.0
            ));
        }

        let hour = metric.timestamp.hour();
        if let Some(expected) = baseline.seasonal_patterns.hourly_patterns.get(&hour) {
            let hour_deviation = ((metric.value - expected) / expected).abs();
            if hour_deviation > 0.5 {
                factors.push(format!(
                    "Deviates {:.1}% from typical hour-of-day pattern",
                    hour_deviation * 100.0
                ));
            }
        }

        if baseline.stats.sample_count < 500 {
            factors.push("Limited baseline data (early learning phase)".to_string());
        }

        factors
    }
}
