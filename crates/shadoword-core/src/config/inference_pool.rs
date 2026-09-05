use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashSet;

pub(crate) const DEFAULT_GPU_HOST_THREADS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionTarget {
    Cpu {
        #[serde(default)]
        #[specta(type = Option<u32>)]
        threads: Option<usize>,
    },
    Gpu {
        device: i32,
        #[serde(default)]
        #[specta(type = Option<u32>)]
        host_threads: Option<usize>,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExecutionUnitConfig {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub required: bool,
    pub target: ExecutionTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct InferenceLimits {
    #[specta(type = u32)]
    pub max_queued_jobs: usize,
    #[specta(type = u32)]
    pub max_queued_audio_bytes: usize,
    #[specta(type = u32)]
    pub max_audio_bytes_per_job: usize,
    #[specta(type = u32)]
    pub max_outstanding_per_flow: usize,
    #[specta(type = u32)]
    #[serde(rename = "max_buffered_results_per_flow")]
    // Sequencer-only bound for API stream completions waiting on an earlier
    // sequence. Core job receivers are single-result channels are not counted.
    // The old wire name is retained for config compatibility.
    pub max_sequencer_buffered_results_per_flow: usize,
}

impl Default for InferenceLimits {
    fn default() -> Self {
        Self {
            max_queued_jobs: 32,
            max_queued_audio_bytes: 64 * 1024 * 1024,
            max_audio_bytes_per_job: 64 * 1024 * 1024,
            max_outstanding_per_flow: 8,
            max_sequencer_buffered_results_per_flow: 32,
        }
    }
}

fn default_preload_timeout_ms() -> u64 {
    120_000
}

fn default_max_draining_generations() -> usize {
    2
}

const MAX_PRELOAD_TIMEOUT_MS: u64 = 30 * 60 * 1_000;
const MAX_DRAINING_GENERATIONS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct InferencePoolConfig {
    pub units: Vec<ExecutionUnitConfig>,
    pub limits: InferenceLimits,
    #[specta(type = f64)]
    pub preload_timeout_ms: u64,
    #[specta(type = u32)]
    pub max_draining_generations: usize,
}

impl Default for InferencePoolConfig {
    fn default() -> Self {
        Self {
            units: Vec::new(),
            limits: InferenceLimits::default(),
            preload_timeout_ms: default_preload_timeout_ms(),
            max_draining_generations: default_max_draining_generations(),
        }
    }
}

impl InferencePoolConfig {
    pub fn validate(&self) -> Result<()> {
        let parallelism = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);
        self.validate_with_parallelism(parallelism)
    }

    pub fn validate_with_parallelism(&self, available_parallelism: usize) -> Result<()> {
        if self.limits.max_queued_audio_bytes == 0
            || self.limits.max_audio_bytes_per_job == 0
            || self.limits.max_outstanding_per_flow == 0
            || self.limits.max_sequencer_buffered_results_per_flow == 0
        {
            anyhow::bail!(
                "inference pool byte, flow, and sequencer limits must be greater than zero"
            );
        }
        if self.preload_timeout_ms == 0 {
            anyhow::bail!("inference pool preload_timeout_ms must be greater than zero");
        }
        if self.preload_timeout_ms > MAX_PRELOAD_TIMEOUT_MS {
            anyhow::bail!(
                "inference pool preload_timeout_ms must not exceed {MAX_PRELOAD_TIMEOUT_MS}"
            );
        }
        if self.max_draining_generations == 0 {
            anyhow::bail!("inference pool max_draining_generations must be greater than zero");
        }
        if self.max_draining_generations > MAX_DRAINING_GENERATIONS {
            anyhow::bail!(
                "inference pool max_draining_generations must not exceed {MAX_DRAINING_GENERATIONS}"
            );
        }

        let enabled = self
            .units
            .iter()
            .filter(|unit| unit.enabled)
            .collect::<Vec<_>>();
        if enabled.is_empty() {
            anyhow::bail!("inference pool must contain at least one enabled execution unit");
        }

        let mut ids = HashSet::new();
        for unit in &self.units {
            if !valid_unit_id(&unit.id) {
                anyhow::bail!(
                    "invalid execution unit id {:?}; use 1-64 ASCII letters, digits, '.', '_' or '-'",
                    unit.id
                );
            }
            if !ids.insert(unit.id.as_str()) {
                anyhow::bail!("duplicate execution unit id {:?}", unit.id);
            }
        }

        let mut gpu_devices = HashSet::new();
        let mut requested_cpu_threads = 0usize;
        let available_parallelism = available_parallelism.max(1);

        for unit in enabled {
            match unit.target {
                ExecutionTarget::Gpu {
                    device,
                    host_threads,
                } => {
                    if device < 0 {
                        anyhow::bail!(
                            "execution unit {:?} must use an explicit non-negative GPU device",
                            unit.id
                        );
                    }
                    if !gpu_devices.insert(device) {
                        anyhow::bail!("GPU device {device} is assigned to more than one unit");
                    }
                    let threads = host_threads.unwrap_or(DEFAULT_GPU_HOST_THREADS);
                    if threads == 0 || threads > available_parallelism {
                        anyhow::bail!(
                            "execution unit {:?} requests {threads} GPU host threads, but available parallelism is {available_parallelism}",
                            unit.id
                        );
                    }
                    requested_cpu_threads = requested_cpu_threads.saturating_add(threads);
                }
                ExecutionTarget::Cpu { threads } => {
                    let threads = threads.unwrap_or(available_parallelism.min(4));
                    if threads == 0 || threads > available_parallelism {
                        anyhow::bail!(
                            "execution unit {:?} requests {threads} CPU threads, but available parallelism is {available_parallelism}",
                            unit.id
                        );
                    }
                    requested_cpu_threads = requested_cpu_threads.saturating_add(threads);
                }
            }
        }

        if requested_cpu_threads > available_parallelism {
            anyhow::bail!(
                "CPU execution units request {requested_cpu_threads} total threads, exceeding available parallelism {available_parallelism}"
            );
        }
        Ok(())
    }
}

fn valid_unit_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
