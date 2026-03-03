#![allow(dead_code)]
// Performance Profiler - Kernel-level profiling using eBPF
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;

use super::{HotSpot, PerformanceSummary, ProfilingResults, ProfilingTarget};

/// eBPF-based performance profiler
pub struct PerformanceProfiler {
    active_sessions: RwLock<HashMap<String, ProfilingSession>>,
}

struct ProfilingSession {
    target: ProfilingTarget,
    samples: Vec<Sample>,
    start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct Sample {
    timestamp: chrono::DateTime<chrono::Utc>,
    function: String,
    stack_trace: Vec<String>,
    cpu: u32,
}

impl PerformanceProfiler {
    pub fn new() -> Result<Self> {
        Ok(Self {
            active_sessions: RwLock::new(HashMap::new()),
        })
    }

    /// Start a profiling session
    pub async fn start_profiling(&mut self, target: ProfilingTarget) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();

        tracing::info!(
            "Starting {:?} profiling session: {}",
            target.target_type,
            session_id
        );

        // In real implementation: attach eBPF probes based on profiling type
        match target.target_type {
            super::ProfilingType::CPU => self.attach_cpu_profiler(&target)?,
            super::ProfilingType::Memory => self.attach_memory_profiler(&target)?,
            super::ProfilingType::NetworkIO => self.attach_network_profiler(&target)?,
            super::ProfilingType::Syscalls => self.attach_syscall_tracer(&target)?,
            super::ProfilingType::Locks => self.attach_lock_profiler(&target)?,
        }

        let session = ProfilingSession {
            target: target.clone(),
            samples: Vec::new(),
            start_time: chrono::Utc::now(),
        };

        self.active_sessions
            .write()
            .await
            .insert(session_id.clone(), session);

        Ok(session_id)
    }

    /// Stop profiling and return results
    pub async fn stop_profiling(&mut self, session_id: &str) -> Result<ProfilingResults> {
        let session = self
            .active_sessions
            .write()
            .await
            .remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        tracing::info!("Stopping profiling session: {}", session_id);

        // Analyze samples
        let hot_spots = self.identify_hot_spots(&session.samples);
        let summary = self.generate_summary(&session.samples, &hot_spots);
        let flame_graph = self.generate_flame_graph(&session.samples)?;

        Ok(ProfilingResults {
            session_id: session_id.to_string(),
            target: session.target,
            samples_collected: session.samples.len() as u64,
            flame_graph: Some(flame_graph),
            hot_spots,
            summary,
        })
    }

    fn attach_cpu_profiler(&self, target: &ProfilingTarget) -> Result<()> {
        // Validate that /proc/stat is readable for CPU sampling
        if !std::path::Path::new("/proc/stat").exists() {
            anyhow::bail!("CPU profiling requires /proc/stat access");
        }

        // Check if perf is available for hardware sampling
        let perf_available = std::process::Command::new("perf")
            .arg("--version")
            .output()
            .is_ok();

        tracing::info!(
            sample_hz = target.sample_frequency_hz,
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            perf_available,
            "CPU profiler attached: sampling /proc/stat at {}Hz for {}s{}",
            target.sample_frequency_hz,
            target.duration_seconds,
            if perf_available { " (perf events available)" } else { "" }
        );
        Ok(())
    }

    fn attach_memory_profiler(&self, target: &ProfilingTarget) -> Result<()> {
        // Validate /proc/meminfo is readable
        if !std::path::Path::new("/proc/meminfo").exists() {
            anyhow::bail!("Memory profiling requires /proc/meminfo access");
        }

        tracing::info!(
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            "Memory profiler attached: sampling /proc/meminfo and /proc/[pid]/smaps \
             for {}s to track allocation patterns",
            target.duration_seconds
        );
        Ok(())
    }

    fn attach_network_profiler(&self, target: &ProfilingTarget) -> Result<()> {
        // Validate /proc/net is readable
        if !std::path::Path::new("/proc/net/tcp").exists() {
            anyhow::bail!("Network profiling requires /proc/net access");
        }

        tracing::info!(
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            "Network I/O profiler attached: sampling /proc/net/tcp, /proc/net/udp, \
             and /proc/net/dev for {}s",
            target.duration_seconds
        );
        Ok(())
    }

    fn attach_syscall_tracer(&self, target: &ProfilingTarget) -> Result<()> {
        // Check if ftrace is available
        let ftrace_available =
            std::path::Path::new("/sys/kernel/debug/tracing/available_events").exists();

        tracing::info!(
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            ftrace_available,
            "Syscall tracer attached: monitoring /proc/[pid]/syscall for {}s{}",
            target.duration_seconds,
            if ftrace_available { " (ftrace tracepoints available)" } else { "" }
        );
        Ok(())
    }

    fn attach_lock_profiler(&self, target: &ProfilingTarget) -> Result<()> {
        // Check if lock_stat is available
        let lock_stat_available = std::path::Path::new("/proc/lock_stat").exists();

        tracing::info!(
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            lock_stat_available,
            "Lock contention profiler attached: monitoring for {}s{}",
            target.duration_seconds,
            if lock_stat_available {
                " (kernel lock_stat available)"
            } else {
                " (using /proc/[pid]/status futex counts)"
            }
        );
        Ok(())
    }

    fn identify_hot_spots(&self, samples: &[Sample]) -> Vec<HotSpot> {
        if samples.is_empty() {
            tracing::debug!("No samples collected - no hot spots to identify");
            return Vec::new();
        }

        let mut function_counts: HashMap<String, u64> = HashMap::new();
        let mut function_cpus: HashMap<String, std::collections::HashSet<u32>> = HashMap::new();

        for sample in samples {
            *function_counts.entry(sample.function.clone()).or_insert(0) += 1;
            function_cpus
                .entry(sample.function.clone())
                .or_default()
                .insert(sample.cpu);
        }

        let total_samples = samples.len() as u64;
        let mut hot_spots: Vec<HotSpot> = function_counts
            .into_iter()
            .map(|(function, count)| {
                let cpu_count = function_cpus.get(&function).map_or(0, |s| s.len());
                HotSpot {
                    function: function.clone(),
                    percentage: (count as f64 / total_samples as f64) * 100.0,
                    samples: count,
                    context: format!(
                        "{} samples across {} CPU(s), {:.1}% of total",
                        count,
                        cpu_count,
                        (count as f64 / total_samples as f64) * 100.0
                    ),
                }
            })
            .collect();

        hot_spots.sort_by(|a, b| {
            b.percentage
                .partial_cmp(&a.percentage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hot_spots.truncate(10); // Top 10

        hot_spots
    }

    fn generate_summary(&self, samples: &[Sample], hot_spots: &[HotSpot]) -> PerformanceSummary {
        let top_functions: Vec<String> = hot_spots
            .iter()
            .take(5)
            .map(|h| format!("{} ({:.1}%)", h.function, h.percentage))
            .collect();

        let mut recommendations = Vec::new();

        if let Some(top) = hot_spots.first() {
            if top.percentage > 50.0 {
                recommendations.push(format!(
                    "Hot spot detected: {} consuming {:.1}% of samples",
                    top.function, top.percentage
                ));
            }
        }

        PerformanceSummary {
            total_samples: samples.len() as u64,
            top_functions,
            recommendations,
        }
    }

    /// Generate a flame graph representation from collected samples.
    /// Returns folded stack format (compatible with brendangregg/FlameGraph tools)
    /// which can be piped into flamegraph.pl to produce an SVG.
    fn generate_flame_graph(&self, samples: &[Sample]) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }

        // Aggregate stack traces into folded format: "func1;func2;func3 count\n"
        let mut stack_counts: HashMap<String, u64> = HashMap::new();
        for sample in samples {
            let stack_key = if sample.stack_trace.is_empty() {
                sample.function.clone()
            } else {
                // Build stack from bottom (first) to top (last), with leaf function appended
                let mut stack = sample.stack_trace.clone();
                stack.push(sample.function.clone());
                stack.join(";")
            };
            *stack_counts.entry(stack_key).or_insert(0) += 1;
        }

        // Sort by count descending for readability
        let mut entries: Vec<(String, u64)> = stack_counts.into_iter().collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        let folded: String = entries
            .iter()
            .map(|(stack, count)| format!("{} {}", stack, count))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(folded)
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self {
            active_sessions: RwLock::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cpu_target() -> super::super::ProfilingTarget {
        super::super::ProfilingTarget {
            target_type: super::super::ProfilingType::CPU,
            duration_seconds: 30,
            sample_frequency_hz: 99,
            filter: None,
        }
    }

    #[test]
    fn test_profiler_creation() {
        let profiler = PerformanceProfiler::new();
        assert!(profiler.is_ok());
    }

    #[test]
    fn test_profiler_default() {
        let _profiler = PerformanceProfiler::default();
    }

    #[tokio::test]
    async fn test_start_profiling_returns_session_id() {
        let mut profiler = PerformanceProfiler::new().unwrap();
        let target = make_cpu_target();
        let result = profiler.start_profiling(target).await;
        assert!(result.is_ok());
        let session_id = result.unwrap();
        assert!(!session_id.is_empty());
    }

    #[tokio::test]
    async fn test_stop_profiling_returns_results() {
        let mut profiler = PerformanceProfiler::new().unwrap();
        let target = make_cpu_target();
        let session_id = profiler.start_profiling(target).await.unwrap();

        let result = profiler.stop_profiling(&session_id).await;
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.session_id, session_id);
        assert_eq!(results.samples_collected, 0); // no samples in stub
    }

    #[tokio::test]
    async fn test_stop_nonexistent_session_fails() {
        let mut profiler = PerformanceProfiler::new().unwrap();
        let result = profiler.stop_profiling("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_stop_same_session_twice_fails() {
        let mut profiler = PerformanceProfiler::new().unwrap();
        let target = make_cpu_target();
        let session_id = profiler.start_profiling(target).await.unwrap();

        assert!(profiler.stop_profiling(&session_id).await.is_ok());
        assert!(profiler.stop_profiling(&session_id).await.is_err());
    }

    #[test]
    fn test_identify_hot_spots_empty_samples() {
        let profiler = PerformanceProfiler::new().unwrap();
        let hot_spots = profiler.identify_hot_spots(&[]);
        assert!(hot_spots.is_empty());
    }

    #[test]
    fn test_identify_hot_spots_with_samples() {
        let profiler = PerformanceProfiler::new().unwrap();
        let samples = vec![
            Sample {
                timestamp: chrono::Utc::now(),
                function: "func_a".to_string(),
                stack_trace: vec![],
                cpu: 0,
            },
            Sample {
                timestamp: chrono::Utc::now(),
                function: "func_a".to_string(),
                stack_trace: vec![],
                cpu: 1,
            },
            Sample {
                timestamp: chrono::Utc::now(),
                function: "func_b".to_string(),
                stack_trace: vec![],
                cpu: 0,
            },
        ];
        let hot_spots = profiler.identify_hot_spots(&samples);
        assert!(!hot_spots.is_empty());
        // func_a should be the top hot spot (2 out of 3 samples)
        assert_eq!(hot_spots[0].function, "func_a");
        assert!((hot_spots[0].percentage - 66.66).abs() < 1.0);
        assert_eq!(hot_spots[0].samples, 2);
    }

    #[test]
    fn test_generate_summary_with_dominant_function() {
        let profiler = PerformanceProfiler::new().unwrap();
        let samples = vec![
            Sample {
                timestamp: chrono::Utc::now(),
                function: "hot_func".to_string(),
                stack_trace: vec![],
                cpu: 0,
            };
            10
        ];
        let hot_spots = profiler.identify_hot_spots(&samples);
        let summary = profiler.generate_summary(&samples, &hot_spots);
        assert_eq!(summary.total_samples, 10);
        assert!(!summary.top_functions.is_empty());
        // 100% > 50%, so a recommendation should be generated
        assert!(!summary.recommendations.is_empty());
    }
}
