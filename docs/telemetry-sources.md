# Telemetry source mapping

`Measured` means the OS supplies the field. `Derived` means the backend computes a rate or ratio from measured cumulative counters. Missing capability or permission is represented as null/unavailable, never zero.

| Schema field | Source | Unit / behavior | Measured or derived | Privilege | API status and dependencies |
| --- | --- | --- | --- | --- | --- |
| `process.identity.pid/ppid/uid/name/processStartTime` | `proc_pidinfo(PROC_PIDTBSDINFO)` | IDs, UTF-8 name, Unix seconds | Measured snapshot | No | Public libproc; process may exit |
| `process.identity.id` | PID plus process start seconds/microseconds | String | Derived identity | No | Prevents PID reuse collisions |
| `process.identity.executable` | `proc_pidpath` | Path | Measured snapshot | No | Public libproc; EPERM/ESRCH possible |
| `process.identity.bundleIdentifier` | No reliable active source | — | Unavailable | No | Null |
| `process.identity.responsiblePid/coalitionId` | powermetrics when safely enabled | IDs | Unavailable in this build | Would require helper | Null |
| `process.cpu.userTimeNs/systemTimeNs` | current rusage | ns, cumulative | Measured | No | Public libproc; current SDK field |
| `process.cpu.percent` | CPU-time delta / wall-time delta | percent, may exceed 100 on multicore | Derived | No | Reset rejects the rate |
| `process.cpu.instructions/cycles` | current rusage | count, cumulative | Measured | No | V4+ |
| `process.cpu.instructionsPerSecond/cyclesPerSecond` | counter delta / wall time | count/s | Derived | No | V4+ |
| `process.cpu.instructionsPerCycle` | instruction delta / cycle delta | ratio | Derived | No | Null when cycle delta is zero |
| `process.cpu.pCoreInstructions/pCoreCycles` | current rusage | count, cumulative | Measured | No | V6+ only |
| `process.cpu.runnableTimeNs` | current rusage | ns, cumulative | Measured | No | V4+ |
| `process.gpu.accumulatedTimeNs` | AGX IORegistry `AppUsage.accumulatedGPUTime` | ns, cumulative under observed schema | Measured | No | Private/undocumented; Apple Silicon/schema dependent |
| `process.gpu.deltaTimeNs/utilizationPercent` | AGX counter delta and wall time | ns / percent | Derived | No | Null on reset/missing attribution |
| `process.neural.*FootprintBytes` | current rusage | bytes | Measured | No | V6+; not ANE execution/utilization |
| `process.memory.residentBytes/physicalFootprintBytes/wiredBytes/pageins` | current rusage | bytes or cumulative pages | Measured | No | Public libproc |
| `process.memory.maxPhysicalFootprintBytes` | current rusage lifetime maximum | bytes | Measured | No | V4+ |
| `process.diskIo.readBytes/writeBytes/logicalWrites` | current rusage | bytes/count, cumulative | Measured | No | Public libproc |
| `process.diskIo.*BytesPerSecond` | rusage byte delta / wall time | bytes/s | Derived | No | Null on counter reset |
| `process.network.*` | powermetrics process netstats | bytes / bytes/s | Unavailable in this build | Would require helper | `available=false`; never estimated |
| `process.scheduling.interruptWakeups/packageIdleWakeups` | current rusage | count, cumulative | Measured | No | Public libproc |
| `process.scheduling.*WakeupsPerSecond` | wakeup delta / wall time | count/s | Derived | No | Null on reset |
| `process.scheduling.waitTimeNs` | powermetrics process waits/SFI | ns | Unavailable in this build | Would require helper | Null |
| `process.qos.*CpuNs` | current rusage | ns, cumulative by QoS | Measured | No | V3+ |
| `process.energy.billedEnergyNj/servicedEnergyNj` | current rusage legacy energy fields | kernel accounting unit exposed by field | Measured | No | V4+; named to schema, no invented conversion |
| `process.energy.energyNj/pCoreEnergyNj` | current rusage | nJ, cumulative | Measured | No | V6+ |
| `process.energy.energyImpact` | powermetrics | score | Unavailable in this build | Would require helper | Null |
| `system.cpu.user/system/idle/utilizationPercent` | Mach host CPU ticks | percent per sample window | Derived from measured cumulative ticks | No | Public Mach API |
| `system.cpu.frequency/residency/power/instructions/cycles` | powermetrics | MHz/ratio/residency/mW/count/s | Unavailable in this build | Would require helper | Null; no fabricated global clock |
| `system.memory.physicalBytes` | read-only `hw.memsize` sysctl | bytes | Measured | No | Public sysctl read |
| `system.memory.free/active/inactive/wired/compressedBytes` | Mach `HOST_VM_INFO64` × page size | bytes | Derived conversion from measured pages | No | Public Mach API |
| `system.memory.usedBytes` | active + inactive + wired + compressed, capped at physical | bytes | Derived | No | Formula recorded here |
| `system.memory.swapUsedBytes` | read-only `vm.swapusage` sysctl | bytes | Measured | No | Public sysctl read |
| `system.memory.pageins/pageoutsPerSecond` | VM counter delta / wall time | pages/s | Derived | No | Null on reset |
| `system.memory.memoryPressure` | No current supported source | — | Unavailable | No | Null |
| `system.disk.*` | IORegistry `IOBlockStorageDriver/Statistics` | bytes/s and ops/s | Derived from measured cumulative properties | No | Public observation path; property presence varies |
| `system.network.*` | `getifaddrs` `if_data` counters | bytes/s and packets/s | Derived from cumulative counters | No | Public; near-maximum 32-bit wrap handled |
| `system.scheduler.*WakeupsPerSecond` | sum of visible process rusage rates | count/s | Derived partial aggregate | No | Permission-dependent; capability context accompanies event |
| `system.scheduler.forcedIdlePercent` | powermetrics SFI | percent | Unavailable in this build | Would require helper | Null |
| `system.gpu.*` | powermetrics GPU sampler | percent/MHz/residency/mW | Unavailable in this build | Would require helper | Null; separate from AGX process GPU |
| `system.ane.*` | powermetrics ANE sampler | percent/residency/mW/mJ when emitted | Unavailable in this build | Would require helper | Null; no utilization inferred from power |
| `system.thermal.*` | powermetrics thermal/plimits | state/pressure/percent/boolean | Unavailable in this build | Would require helper | Null unless exact fields are emitted |
