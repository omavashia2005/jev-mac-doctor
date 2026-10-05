mod ioreg;
#[cfg(target_os = "macos")]
mod macos;
mod powermetrics;

use crate::model::*;
use ioreg::DiskCounters;
use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

const UNAVAILABLE_ANE_REASON: &str =
    "No reliable PID-level ANE execution attribution is exposed by macOS";

#[derive(Clone, Copy, Debug, Default)]
struct PreviousProcess {
    timestamp_ns: u64,
    cpu_ns: u64,
    instructions: u64,
    cycles: u64,
    read_bytes: u64,
    write_bytes: u64,
    interrupt_wakeups: u64,
    package_idle_wakeups: u64,
    gpu_time_ns: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
struct PreviousSystem {
    timestamp_ns: u64,
    cpu: macos::CpuTicks,
    vm: macos::VmCounters,
    network: macos::NetworkCounters,
    disk: Option<DiskCounters>,
}

pub struct TelemetryCollector {
    previous_processes: HashMap<String, PreviousProcess>,
    previous_system: Option<PreviousSystem>,
    capabilities: BTreeMap<String, Capability>,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        let mut capabilities = BTreeMap::new();
        capabilities.insert(
            "libproc_process_enumeration".into(),
            capability(true, "libproc", None, false, false),
        );
        capabilities.insert(
            "rusage_info_current".into(),
            capability(
                true,
                "proc_pid_rusage / RUSAGE_INFO_CURRENT",
                None,
                false,
                false,
            ),
        );
        capabilities.insert(
            "system_memory".into(),
            capability(
                true,
                "Mach host_statistics64 + read-only sysctl",
                None,
                false,
                false,
            ),
        );
        capabilities.insert(
            "system_network".into(),
            capability(true, "getifaddrs interface counters", None, false, false),
        );
        capabilities.insert(
            "per_process_ane_execution".into(),
            capability(false, "macOS", Some(UNAVAILABLE_ANE_REASON), false, false),
        );
        capabilities.insert(
            "process_network".into(),
            capability(
                false,
                "powermetrics",
                Some("PID network accounting needs an authorized powermetrics collector"),
                false,
                true,
            ),
        );
        let powermetrics = powermetrics::detect();
        capabilities.insert(
            "powermetrics".into(),
            capability(
                false,
                "powermetrics plist",
                Some(if powermetrics.installed && powermetrics.plist_supported {
                    "Detected, but disabled without a separately authorized read-only privileged helper"
                } else {
                    "Not installed or this version does not advertise plist output"
                }),
                false,
                true,
            ),
        );
        capabilities.insert(
            "system_gpu".into(),
            capability(
                false,
                "powermetrics",
                Some("powermetrics collector unavailable"),
                false,
                true,
            ),
        );
        capabilities.insert(
            "system_ane_power".into(),
            capability(
                false,
                "powermetrics",
                Some("powermetrics collector unavailable"),
                false,
                true,
            ),
        );
        capabilities.insert(
            "thermal_power_limits".into(),
            capability(
                false,
                "powermetrics",
                Some("Exact power-limit fields are hardware/macOS dependent and not emitted"),
                false,
                true,
            ),
        );
        Self {
            previous_processes: HashMap::new(),
            previous_system: None,
            capabilities,
        }
    }

    pub fn capabilities(&self) -> BTreeMap<String, Capability> {
        self.capabilities.clone()
    }

    pub fn sample(&mut self) -> Result<TelemetryFrame, Vec<DiagnosticError>> {
        let timestamp_ns = now_ns();
        let native_processes = macos::processes()
            .map_err(|message| vec![diagnostic_error(timestamp_ns, "libproc", message, true)])?;
        let current_rusage = native_processes
            .iter()
            .any(|process| process.rusage_version == macos::ffi::RUSAGE_INFO_CURRENT);
        self.capabilities.insert(
            "rusage_info_current".into(),
            capability(
                current_rusage,
                "proc_pid_rusage / RUSAGE_INFO_CURRENT",
                (!current_rusage).then_some("Runtime rejected the SDK current flavor; using a read-only V5/V4 prefix fallback"),
                false,
                false,
            ),
        );
        let rusage_v6 = native_processes
            .iter()
            .any(|process| process.rusage_version >= macos::ffi::RUSAGE_INFO_V6);
        self.capabilities.insert(
            "process_neural_footprint".into(),
            capability(
                rusage_v6,
                "proc_pid_rusage / RUSAGE_INFO_CURRENT",
                (!rusage_v6)
                    .then_some("The running macOS did not return a V6 neural-footprint field"),
                false,
                false,
            ),
        );
        let cpu = macos::cpu_ticks()
            .map_err(|message| vec![diagnostic_error(timestamp_ns, "mach-cpu", message, true)])?;
        let vm = macos::vm_counters()
            .map_err(|message| vec![diagnostic_error(timestamp_ns, "mach-vm", message, true)])?;
        let network = macos::network_counters().unwrap_or_default();

        let agx = ioreg::agx_gpu_times();
        match &agx {
            Ok(_) => self.capabilities.insert(
                "per_process_gpu".into(),
                capability(
                    true,
                    "AGX IORegistry AppUsage.accumulatedGPUTime",
                    None,
                    true,
                    false,
                ),
            ),
            Err(reason) => self.capabilities.insert(
                "per_process_gpu".into(),
                capability(false, "AGX IORegistry", Some(reason), true, false),
            ),
        };

        let disk = ioreg::storage_counters();
        match &disk {
            Ok(_) => self.capabilities.insert(
                "system_disk".into(),
                capability(
                    true,
                    "IOBlockStorageDriver Statistics via IORegistry plist read",
                    None,
                    false,
                    false,
                ),
            ),
            Err(reason) => self.capabilities.insert(
                "system_disk".into(),
                capability(false, "IORegistry", Some(reason), false, false),
            ),
        };

        let gpu_times = agx.unwrap_or_default();
        let mut next_previous = HashMap::with_capacity(native_processes.len());
        let mut processes = Vec::with_capacity(native_processes.len());
        for native in native_processes {
            let start =
                native.start_seconds as f64 + native.start_microseconds as f64 / 1_000_000.0;
            let id = format!(
                "{}:{}.{:06}",
                native.pid, native.start_seconds, native.start_microseconds
            );
            let identity = ProcessIdentity {
                id: id.clone(),
                pid: native.pid,
                ppid: Some(native.ppid),
                process_start_time: Some(start),
                name: native.name,
                executable: native.executable,
                bundle_identifier: None,
                responsible_pid: None,
                coalition_id: None,
                uid: Some(native.uid),
            };
            let usage = native.usage;
            let rusage_v6 = native.rusage_version >= macos::ffi::RUSAGE_INFO_V6;
            let current = PreviousProcess {
                timestamp_ns,
                cpu_ns: total_cpu_time(&usage),
                instructions: usage.ri_instructions,
                cycles: usage.ri_cycles,
                read_bytes: usage.ri_diskio_bytesread,
                write_bytes: usage.ri_diskio_byteswritten,
                interrupt_wakeups: usage.ri_interrupt_wkups,
                package_idle_wakeups: usage.ri_pkg_idle_wkups,
                gpu_time_ns: gpu_times.get(&native.pid).copied(),
            };
            let previous = self.previous_processes.get(&id).copied();
            let elapsed_seconds = previous
                .map(|value| timestamp_ns.saturating_sub(value.timestamp_ns) as f64 / 1e9)
                .filter(|value| *value > 0.0);
            let rate = |current: u64, old: fn(PreviousProcess) -> u64| {
                previous
                    .and_then(|value| counter_delta(current, old(value)))
                    .zip(elapsed_seconds)
                    .map(|(delta, seconds)| delta as f64 / seconds)
            };
            let instructions_per_second = rate(usage.ri_instructions, |p| p.instructions);
            let cycles_per_second = rate(usage.ri_cycles, |p| p.cycles);
            let gpu_delta = previous
                .and_then(|value| value.gpu_time_ns)
                .zip(current.gpu_time_ns)
                .and_then(|(old, current)| counter_delta(current, old));
            let process = ProcessSample {
                timestamp_ns,
                identity,
                cpu: ProcessCpu {
                    percent: rate(current.cpu_ns, |p| p.cpu_ns).map(|value| value / 1e7),
                    user_time_ns: Some(usage.ri_user_time),
                    system_time_ns: Some(usage.ri_system_time),
                    runnable_time_ns: Some(usage.ri_runnable_time),
                    instructions: Some(usage.ri_instructions),
                    instructions_per_second,
                    cycles: Some(usage.ri_cycles),
                    cycles_per_second,
                    instructions_per_cycle: instructions_per_second
                        .zip(cycles_per_second)
                        .filter(|(_, cycles)| *cycles > 0.0)
                        .map(|(instructions, cycles)| instructions / cycles),
                    p_core_instructions: rusage_v6.then_some(usage.ri_pinstructions),
                    p_core_cycles: rusage_v6.then_some(usage.ri_pcycles),
                },
                gpu: ProcessGpu {
                    accumulated_time_ns: current.gpu_time_ns,
                    delta_time_ns: gpu_delta,
                    utilization_percent: gpu_delta
                        .zip(elapsed_seconds)
                        .map(|(delta, seconds)| delta as f64 / (seconds * 1e9) * 100.0),
                    source: current.gpu_time_ns.map(|_| "agx_ioregistry".into()),
                },
                neural: ProcessNeural {
                    footprint_bytes: rusage_v6.then_some(usage.ri_neural_footprint),
                    interval_max_footprint_bytes: rusage_v6
                        .then_some(usage.ri_interval_max_neural_footprint),
                    lifetime_max_footprint_bytes: rusage_v6
                        .then_some(usage.ri_lifetime_max_neural_footprint),
                },
                memory: ProcessMemory {
                    resident_bytes: Some(usage.ri_resident_size),
                    physical_footprint_bytes: Some(usage.ri_phys_footprint),
                    wired_bytes: Some(usage.ri_wired_size),
                    max_physical_footprint_bytes: Some(usage.ri_lifetime_max_phys_footprint),
                    pageins: Some(usage.ri_pageins),
                },
                disk_io: ProcessDiskIo {
                    read_bytes: Some(usage.ri_diskio_bytesread),
                    write_bytes: Some(usage.ri_diskio_byteswritten),
                    read_bytes_per_second: rate(usage.ri_diskio_bytesread, |p| p.read_bytes),
                    write_bytes_per_second: rate(usage.ri_diskio_byteswritten, |p| p.write_bytes),
                    logical_writes: Some(usage.ri_logical_writes),
                },
                network: ProcessNetwork {
                    available: false,
                    ..Default::default()
                },
                scheduling: ProcessScheduling {
                    interrupt_wakeups: Some(usage.ri_interrupt_wkups),
                    interrupt_wakeups_per_second: rate(usage.ri_interrupt_wkups, |p| {
                        p.interrupt_wakeups
                    }),
                    package_idle_wakeups: Some(usage.ri_pkg_idle_wkups),
                    package_idle_wakeups_per_second: rate(usage.ri_pkg_idle_wkups, |p| {
                        p.package_idle_wakeups
                    }),
                    runnable_time_ns: Some(usage.ri_runnable_time),
                    wait_time_ns: None,
                },
                qos: ProcessQos {
                    background_cpu_ns: Some(usage.ri_cpu_time_qos_background),
                    maintenance_cpu_ns: Some(usage.ri_cpu_time_qos_maintenance),
                    utility_cpu_ns: Some(usage.ri_cpu_time_qos_utility),
                    legacy_cpu_ns: Some(usage.ri_cpu_time_qos_legacy),
                    default_cpu_ns: Some(usage.ri_cpu_time_qos_default),
                    user_initiated_cpu_ns: Some(usage.ri_cpu_time_qos_user_initiated),
                    user_interactive_cpu_ns: Some(usage.ri_cpu_time_qos_user_interactive),
                },
                energy: ProcessEnergy {
                    energy_nj: rusage_v6.then_some(usage.ri_energy_nj),
                    billed_energy_nj: Some(usage.ri_billed_energy),
                    serviced_energy_nj: Some(usage.ri_serviced_energy),
                    energy_impact: None,
                    p_core_energy_nj: rusage_v6.then_some(usage.ri_penergy_nj),
                },
            };
            next_previous.insert(id, current);
            processes.push(process);
        }
        self.previous_processes = next_previous;

        let disk = disk.ok();
        let system = build_system_sample(
            timestamp_ns,
            cpu,
            vm,
            network,
            disk,
            self.previous_system,
            &processes,
        );
        self.previous_system = Some(PreviousSystem {
            timestamp_ns,
            cpu,
            vm,
            network,
            disk,
        });
        Ok(TelemetryFrame {
            timestamp_ns,
            system,
            processes,
        })
    }
}

fn build_system_sample(
    timestamp_ns: u64,
    cpu: macos::CpuTicks,
    vm: macos::VmCounters,
    network: macos::NetworkCounters,
    disk: Option<DiskCounters>,
    previous: Option<PreviousSystem>,
    processes: &[ProcessSample],
) -> SystemSample {
    let elapsed = previous
        .map(|value| timestamp_ns.saturating_sub(value.timestamp_ns) as f64 / 1e9)
        .filter(|value| *value > 0.0);
    let cpu_percentages = previous.and_then(|previous| {
        let user = counter_delta(cpu.user, previous.cpu.user)?;
        let system = counter_delta(cpu.system, previous.cpu.system)?;
        let idle = counter_delta(cpu.idle, previous.cpu.idle)?;
        let nice = counter_delta(cpu.nice, previous.cpu.nice)?;
        let total = user + system + idle + nice;
        (total > 0).then(|| {
            (
                (user + nice) as f64 / total as f64 * 100.0,
                system as f64 / total as f64 * 100.0,
                idle as f64 / total as f64 * 100.0,
            )
        })
    });
    let per_second = |current: u64, previous: u64| {
        counter_delta(current, previous)
            .zip(elapsed)
            .map(|(delta, seconds)| delta as f64 / seconds)
    };
    let per_second_u32 = |current: u64, previous: u64| {
        counter_delta_u32(current, previous)
            .zip(elapsed)
            .map(|(delta, seconds)| delta as f64 / seconds)
    };
    let (read_bytes_per_second, write_bytes_per_second, read_ops_per_second, write_ops_per_second) =
        match (disk, previous.and_then(|value| value.disk)) {
            (Some(current), Some(previous)) => (
                per_second(current.read_bytes, previous.read_bytes),
                per_second(current.write_bytes, previous.write_bytes),
                per_second(current.read_ops, previous.read_ops),
                per_second(current.write_ops, previous.write_ops),
            ),
            _ => (None, None, None, None),
        };
    let sum = |value: fn(&ProcessSample) -> Option<f64>| {
        let values = processes.iter().filter_map(value).collect::<Vec<_>>();
        (!values.is_empty()).then(|| values.into_iter().sum())
    };
    let used = vm
        .active_bytes
        .saturating_add(vm.inactive_bytes)
        .saturating_add(vm.wired_bytes)
        .saturating_add(vm.compressed_bytes)
        .min(vm.physical_bytes);
    SystemSample {
        timestamp_ns,
        cpu: SystemCpu {
            utilization_percent: cpu_percentages.map(|(_, _, idle)| 100.0 - idle),
            user_percent: cpu_percentages.map(|(user, _, _)| user),
            system_percent: cpu_percentages.map(|(_, system, _)| system),
            idle_percent: cpu_percentages.map(|(_, _, idle)| idle),
            ..Default::default()
        },
        gpu: SystemGpu::default(),
        ane: SystemAne::default(),
        memory: SystemMemory {
            physical_bytes: Some(vm.physical_bytes),
            used_bytes: Some(used),
            free_bytes: Some(vm.free_bytes),
            active_bytes: Some(vm.active_bytes),
            inactive_bytes: Some(vm.inactive_bytes),
            wired_bytes: Some(vm.wired_bytes),
            compressed_bytes: Some(vm.compressed_bytes),
            swap_used_bytes: Some(vm.swap_used_bytes),
            pageins_per_second: previous
                .map(|p| p.vm.pageins)
                .and_then(|old| per_second(vm.pageins, old)),
            pageouts_per_second: previous
                .map(|p| p.vm.pageouts)
                .and_then(|old| per_second(vm.pageouts, old)),
            memory_pressure: None,
        },
        disk: SystemDisk {
            read_bytes_per_second,
            write_bytes_per_second,
            read_ops_per_second,
            write_ops_per_second,
        },
        network: SystemNetwork {
            rx_bytes_per_second: previous
                .map(|p| p.network.rx_bytes)
                .and_then(|old| per_second_u32(network.rx_bytes, old)),
            tx_bytes_per_second: previous
                .map(|p| p.network.tx_bytes)
                .and_then(|old| per_second_u32(network.tx_bytes, old)),
            rx_packets_per_second: previous
                .map(|p| p.network.rx_packets)
                .and_then(|old| per_second_u32(network.rx_packets, old)),
            tx_packets_per_second: previous
                .map(|p| p.network.tx_packets)
                .and_then(|old| per_second_u32(network.tx_packets, old)),
        },
        scheduler: SystemScheduler {
            interrupt_wakeups_per_second: sum(|p| p.scheduling.interrupt_wakeups_per_second),
            package_idle_wakeups_per_second: sum(|p| p.scheduling.package_idle_wakeups_per_second),
            forced_idle_percent: None,
        },
        thermal: SystemThermal::default(),
    }
}

pub fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .min(u64::MAX as u128) as u64
}

pub fn counter_delta(current: u64, previous: u64) -> Option<u64> {
    (current >= previous).then(|| current - previous)
}

fn counter_delta_u32(current: u64, previous: u64) -> Option<u64> {
    if current >= previous {
        Some(current - previous)
    } else if previous > u32::MAX as u64 - 1_000_000 {
        Some((u32::MAX as u64 - previous) + current + 1)
    } else {
        None
    }
}

fn total_cpu_time(usage: &macos::ffi::rusage_info_v6) -> u64 {
    usage.ri_user_time.saturating_add(usage.ri_system_time)
}

fn capability(
    available: bool,
    source: &str,
    reason: Option<&str>,
    private_api: bool,
    privileged: bool,
) -> Capability {
    Capability {
        available,
        source: Some(source.into()),
        reason: reason.map(str::to_owned),
        private_api,
        privileged,
    }
}

pub fn diagnostic_error(
    timestamp_ns: u64,
    collector: &str,
    message: String,
    recoverable: bool,
) -> DiagnosticError {
    DiagnosticError {
        timestamp_ns,
        collector: collector.into(),
        message,
        recoverable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_reset_is_not_a_giant_delta() {
        assert_eq!(counter_delta(20, 10), Some(10));
        assert_eq!(counter_delta(5, 10), None);
    }

    #[test]
    fn handles_u32_counter_wrap_but_rejects_an_ordinary_reset() {
        assert_eq!(counter_delta_u32(5, u32::MAX as u64 - 2), Some(8));
        assert_eq!(counter_delta_u32(5, 100), None);
    }

    #[test]
    fn maps_current_rusage_cpu_fields() {
        let mut usage = macos::ffi::rusage_info_v6::default();
        usage.ri_user_time = 7;
        usage.ri_system_time = 5;
        assert_eq!(total_cpu_time(&usage), 12);
    }

    #[test]
    fn process_identity_changes_when_pid_is_reused() {
        let first = format!("{}:{}.{:06}", 42, 100, 1);
        let reused = format!("{}:{}.{:06}", 42, 200, 1);
        assert_ne!(first, reused);
    }

    #[test]
    fn capability_map_distinguishes_unavailable_from_zero() {
        let capabilities = TelemetryCollector::new().capabilities();
        assert!(capabilities["libproc_process_enumeration"].available);
        assert!(!capabilities["per_process_ane_execution"].available);
        assert!(capabilities["per_process_ane_execution"].reason.is_some());
    }

    #[test]
    fn native_collector_smoke_test_reads_real_host_state() {
        let mut collector = TelemetryCollector::new();
        for _ in 0..10 {
            let frame = collector.sample().unwrap();
            assert!(!frame.processes.is_empty());
            assert!(frame.system.memory.physical_bytes.unwrap_or_default() > 0);
        }
    }
}
