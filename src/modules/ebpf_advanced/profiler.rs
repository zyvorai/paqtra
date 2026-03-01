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
        tracing::info!(
            sample_hz = target.sample_frequency_hz,
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            "STUB: Would attach CPU profiler via perf_event eBPF program. \
             In production: attach BPF_PROG_TYPE_PERF_EVENT to sample CPU stacks \
             at {}Hz for {}s.",
            target.sample_frequency_hz,
            target.duration_seconds
        );
        Ok(())
    }

    fn attach_memory_profiler(&self, target: &ProfilingTarget) -> Result<()> {
        tracing::info!(
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            "STUB: Would attach memory profiler via uprobe/kprobe. \
             In production: attach to malloc/free/mmap/munmap to track allocations \
             and detect leaks."
        );
        Ok(())
    }

    fn attach_network_profiler(&self, target: &ProfilingTarget) -> Result<()> {
        tracing::info!(
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            "STUB: Would attach network I/O profiler via tracepoints. \
             In production: attach to tcp_sendmsg/tcp_recvmsg and socket syscalls."
        );
        Ok(())
    }

    fn attach_syscall_tracer(&self, target: &ProfilingTarget) -> Result<()> {
        tracing::info!(
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            "STUB: Would attach syscall tracer via tracepoints. \
             In production: attach to raw_syscalls:sys_enter and raw_syscalls:sys_exit."
        );
        Ok(())
    }

    fn attach_lock_profiler(&self, target: &ProfilingTarget) -> Result<()> {
        tracing::info!(
            duration_secs = target.duration_seconds,
            filter = ?target.filter,
            "STUB: Would attach lock contention profiler via kprobes. \
             In production: attach to mutex_lock/mutex_unlock and measure contention."
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

        hot_spots.sort_by(|a, b| b.percentage.partial_cmp(&a.percentage).unwrap_or(std::cmp::Ordering::Equal));
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
