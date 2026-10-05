# Read-only native and system-call audit

Audit date: 2026-09-17. The telemetry layer is an observer. Every system-mutation answer below is **No**.

| API or operation | Purpose | Read or write | Privilege | Can affect another process? | Can affect persistent system state? |
| --- | --- | --- | --- | --- | --- |
| `proc_listallpids` | Enumerate process IDs | Read | No | No | No |
| `proc_pidinfo(PROC_PIDTBSDINFO)` | Read PID, PPID, UID, name, and start time | Read | No; visibility may be limited | No | No |
| `proc_pidpath` | Read executable path | Read | No; may return EPERM | No | No |
| `proc_pid_rusage(RUSAGE_INFO_CURRENT)` | Read cumulative CPU, memory, IO, wakeup, QoS, instruction, cycle, energy, runnable, and neural-footprint counters | Read | No; visibility may be limited | No | No |
| `proc_pid_rusage(RUSAGE_INFO_V5/V4)` | Runtime compatibility fallback only when current flavor is rejected | Read | No; visibility may be limited | No | No |
| `mach_host_self` | Obtain the host port used by read-only statistic calls | Read capability acquisition | No | No | No |
| `host_statistics(HOST_CPU_LOAD_INFO)` | Read host CPU tick counters | Read | No | No | No |
| `host_statistics64(HOST_VM_INFO64)` | Read VM page counters | Read | No | No | No |
| `host_page_size` | Read VM page size | Read | No | No | No |
| `sysctlbyname("hw.memsize", oldp, oldlenp, null, 0)` | Read physical memory size | Read; `newp` is null | No | No | No |
| `sysctlbyname("vm.swapusage", oldp, oldlenp, null, 0)` | Read swap totals | Read; `newp` is null | No | No | No |
| `getifaddrs` / `freeifaddrs` | Allocate/read/release an interface-counter snapshot | Read and release app memory | No | No | No |
| `/usr/sbin/ioreg -a -r -c AGXAccelerator` | Serialize AGX registry properties for private `AppUsage` observation | Read | No | No | No |
| `/usr/sbin/ioreg -a -r -c IOBlockStorageDriver` | Serialize storage `Statistics` properties | Read | No | No | No |
| `/usr/bin/powermetrics --help` | Detect available flags, samplers, and plist support | Read documentation output | No | No | No |
| TypeSafe HTTPS `POST /v1/systemone` | Send the user-created diagnostic state for typed judgments | Network request | API credential, not root | No | No |
| App-data `create_dir_all`, `write`, `rename`, `read`, `read_dir` | Persist and reopen this app’s own event JSON | Writes only app-owned diagnostic data | No | No | No |
| `.env` read via `dotenvy` | Load the local TypeSafe credential into this process | Read local app-development file | No | No | No |
| Security.framework `SecItemCopyMatching` | Read this app's TypeSafe credential from macOS Keychain | Read app-owned credential | No | No | No |
| Security.framework `SecItemAdd` / `SecItemUpdate` | Save or replace the credential only when the user submits the in-app key form | Write app-owned credential | No | No | No; this writes application data, not system configuration |

## FFI boundaries

Bindings are generated at build time from the installed macOS SDK headers instead of duplicating native structure layouts by hand. Every `unsafe` block in `telemetry/macos.rs` documents the called API, pointer lifetime, buffer size/ownership, result validation, and why the call is read-only.

Pointers returned by `getifaddrs` are used only before `freeifaddrs`. C string data is copied before native buffers are released. Mach and libproc output buffers are interpreted only after success and returned-length checks.

## Privileged surface

None. The app has no helper tool, XPC privileged service, Authorization Services flow, sudo invocation, or generic subprocess endpoint. `powermetrics` sampling remains unavailable instead of expanding the privilege boundary. The entire Tauri process runs as the logged-in user.

The Keychain settings command is separate from telemetry collection. It accepts only a TypeSafe API key and calls the fixed Security.framework generic-password APIs for this app's fixed service/account pair; it cannot execute commands or access arbitrary Keychain entries.

## Prohibited APIs confirmed absent

Repository search confirms no `task_for_pid`, debugger attachment, process suspend/resume/kill, process-memory access, process QoS setter, scheduler setter, IORegistry setter, sysctl setter, `/dev` access, power-management setter, thermal setter, firewall/network configuration, kernel/system extension, SIP/AMFI/NVRAM/firmware operation, or arbitrary command execution.
