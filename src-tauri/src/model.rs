use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessIdentity {
    pub id: String,
    pub pid: i32,
    pub ppid: Option<i32>,
    pub process_start_time: Option<f64>,
    pub name: String,
    pub executable: Option<String>,
    pub bundle_identifier: Option<String>,
    pub responsible_pid: Option<i32>,
    pub coalition_id: Option<u64>,
    pub uid: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessCpu {
    pub percent: Option<f64>,
    pub user_time_ns: Option<u64>,
    pub system_time_ns: Option<u64>,
    pub runnable_time_ns: Option<u64>,
    pub instructions: Option<u64>,
    pub instructions_per_second: Option<f64>,
    pub cycles: Option<u64>,
    pub cycles_per_second: Option<f64>,
    pub instructions_per_cycle: Option<f64>,
    pub p_core_instructions: Option<u64>,
    pub p_core_cycles: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessGpu {
    pub accumulated_time_ns: Option<u64>,
    pub delta_time_ns: Option<u64>,
    pub utilization_percent: Option<f64>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessNeural {
    pub footprint_bytes: Option<u64>,
    pub interval_max_footprint_bytes: Option<u64>,
    pub lifetime_max_footprint_bytes: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMemory {
    pub resident_bytes: Option<u64>,
    pub physical_footprint_bytes: Option<u64>,
    pub wired_bytes: Option<u64>,
    pub max_physical_footprint_bytes: Option<u64>,
    pub pageins: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDiskIo {
    pub read_bytes: Option<u64>,
    pub write_bytes: Option<u64>,
    pub read_bytes_per_second: Option<f64>,
    pub write_bytes_per_second: Option<f64>,
    pub logical_writes: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessNetwork {
    pub rx_bytes: Option<u64>,
    pub tx_bytes: Option<u64>,
    pub rx_bytes_per_second: Option<f64>,
    pub tx_bytes_per_second: Option<f64>,
    pub available: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessScheduling {
    pub interrupt_wakeups: Option<u64>,
    pub interrupt_wakeups_per_second: Option<f64>,
    pub package_idle_wakeups: Option<u64>,
    pub package_idle_wakeups_per_second: Option<f64>,
    pub runnable_time_ns: Option<u64>,
    pub wait_time_ns: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessQos {
    pub background_cpu_ns: Option<u64>,
    pub maintenance_cpu_ns: Option<u64>,
    pub utility_cpu_ns: Option<u64>,
    pub legacy_cpu_ns: Option<u64>,
    pub default_cpu_ns: Option<u64>,
    pub user_initiated_cpu_ns: Option<u64>,
    pub user_interactive_cpu_ns: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEnergy {
    pub energy_nj: Option<u64>,
    pub billed_energy_nj: Option<u64>,
    pub serviced_energy_nj: Option<u64>,
    pub energy_impact: Option<f64>,
    pub p_core_energy_nj: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSample {
    pub timestamp_ns: u64,
    pub identity: ProcessIdentity,
    pub cpu: ProcessCpu,
    pub gpu: ProcessGpu,
    pub neural: ProcessNeural,
    pub memory: ProcessMemory,
    pub disk_io: ProcessDiskIo,
    pub network: ProcessNetwork,
    pub scheduling: ProcessScheduling,
    pub qos: ProcessQos,
    pub energy: ProcessEnergy,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCpu {
    pub utilization_percent: Option<f64>,
    pub user_percent: Option<f64>,
    pub system_percent: Option<f64>,
    pub idle_percent: Option<f64>,
    pub frequency_mhz: Option<f64>,
    pub frequency_ratio: Option<f64>,
    pub performance_core_residency: Option<BTreeMap<String, f64>>,
    pub efficiency_core_residency: Option<BTreeMap<String, f64>>,
    pub instructions_per_second: Option<f64>,
    pub cycles_per_second: Option<f64>,
    pub power_mw: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemGpu {
    pub active_percent: Option<f64>,
    pub frequency_mhz: Option<f64>,
    pub residency: Option<BTreeMap<String, f64>>,
    pub power_mw: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAne {
    pub active_percent: Option<f64>,
    pub residency: Option<BTreeMap<String, f64>>,
    pub power_mw: Option<f64>,
    pub energy_mj: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMemory {
    pub physical_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub active_bytes: Option<u64>,
    pub inactive_bytes: Option<u64>,
    pub wired_bytes: Option<u64>,
    pub compressed_bytes: Option<u64>,
    pub swap_used_bytes: Option<u64>,
    pub pageins_per_second: Option<f64>,
    pub pageouts_per_second: Option<f64>,
    pub memory_pressure: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDisk {
    pub read_bytes_per_second: Option<f64>,
    pub write_bytes_per_second: Option<f64>,
    pub read_ops_per_second: Option<f64>,
    pub write_ops_per_second: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemNetwork {
    pub rx_bytes_per_second: Option<f64>,
    pub tx_bytes_per_second: Option<f64>,
    pub rx_packets_per_second: Option<f64>,
    pub tx_packets_per_second: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemScheduler {
    pub interrupt_wakeups_per_second: Option<f64>,
    pub package_idle_wakeups_per_second: Option<f64>,
    pub forced_idle_percent: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemThermal {
    pub state: Option<String>,
    pub thermal_pressure: Option<String>,
    pub cpu_power_limit_percent: Option<f64>,
    pub gpu_power_limit_percent: Option<f64>,
    pub throttled: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSample {
    pub timestamp_ns: u64,
    pub cpu: SystemCpu,
    pub gpu: SystemGpu,
    pub ane: SystemAne,
    pub memory: SystemMemory,
    pub disk: SystemDisk,
    pub network: SystemNetwork,
    pub scheduler: SystemScheduler,
    pub thermal: SystemThermal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryFrame {
    pub timestamp_ns: u64,
    pub system: SystemSample,
    pub processes: Vec<ProcessSample>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub available: bool,
    pub source: Option<String>,
    pub reason: Option<String>,
    pub private_api: bool,
    pub privileged: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticError {
    pub timestamp_ns: u64,
    pub collector: String,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SlowdownStatus {
    CollectingPostWindow,
    BuildingState,
    QueryingJev,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessWindow {
    pub identity: ProcessIdentity,
    pub samples: Vec<ProcessSample>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JevSeverity {
    pub score: f64,
    pub confidence: Option<f64>,
    pub probabilities: BTreeMap<String, f64>,
    pub legend: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JevPrimaryCause {
    pub process_id: String,
    pub confidence: Option<f64>,
    pub probabilities: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JevMechanism {
    pub choice: String,
    pub confidence: Option<f64>,
    pub probabilities: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JevProcessResult {
    pub process_id: String,
    pub pid: i32,
    pub name: String,
    pub contribution_probability: Option<f64>,
    pub mechanism: Option<JevMechanism>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JevSlowdownResult {
    pub slowdown_severity: JevSeverity,
    pub primary_cause: JevPrimaryCause,
    pub processes: Vec<JevProcessResult>,
    pub raw_response: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlowdownEvent {
    pub id: String,
    pub clicked_at_ns: u64,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub status: SlowdownStatus,
    pub system_samples: Vec<SystemSample>,
    pub processes: Vec<ProcessWindow>,
    pub capabilities: BTreeMap<String, Capability>,
    pub jev_request: Option<Value>,
    pub jev_result: Option<JevSlowdownResult>,
    pub errors: Vec<DiagnosticError>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSummary {
    pub id: String,
    pub clicked_at_ns: u64,
    pub status: SlowdownStatus,
    pub severity: Option<f64>,
    pub primary_process: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorSnapshot {
    pub running: bool,
    pub sample_interval_ms: u64,
    pub processes_tracked: usize,
    pub samples_retained: usize,
    pub last_sample_timestamp_ns: Option<u64>,
    pub latest: Option<TelemetryFrame>,
    pub capabilities: BTreeMap<String, Capability>,
    pub errors: Vec<DiagnosticError>,
}
