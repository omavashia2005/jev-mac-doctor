# Jev Mac Doctor

Jev Mac Doctor is a real Tauri 2 macOS application that continuously records read-only process and system telemetry. When the user clicks **MAC FEELS SLOW**, it preserves the 10 seconds before the click, records the 10 seconds after it, asks TypeSafe/Jev typed questions, and displays the resulting probability distributions.

## Requirements

- macOS on Apple Silicon. The native code is currently macOS-only and is developed on arm64.
- Xcode Command Line Tools or Xcode with the macOS SDK.
- Rust stable and Cargo.
- Node.js 22+ and npm.
- A TypeSafe API key for Jev analysis. Telemetry collection works without a key; captures are retained with a clear Jev error so they can still be inspected.

Intel macOS has not been validated. The libproc and Mach collectors are architecture-neutral, but the private AGX GPU path is specifically useful on Apple Silicon.

## Setup

```bash
npm install
cp .env.example .env
```

Use the **TypeSafe** control in the app header to save the API key in macOS Keychain. For development, you can instead put it in the ignored `.env` file:

```text
TYPESAFE_API_KEY=your_key_here
```

The key value crosses IPC only when the user explicitly saves it, then is written directly to Keychain and cleared from the form. It is never returned to the frontend, logged, or persisted in diagnostic events. A Keychain credential takes precedence over `.env`.

Run in development:

```bash
npm run tauri dev
```

Build the production app:

```bash
npm run tauri build
```

## How capture works

The Rust sampler keeps 60 seconds of native telemetry in memory. A click records an exact nanosecond Unix timestamp `T`, clones samples from `T-10s` through `T`, continues collecting without blocking the UI, then combines samples through `T+10s`.

The complete local event is retained. For Jev, the backend ranks the most relevant processes by observed CPU, GPU, disk, wakeup, and memory signals, creates an explicit compaction manifest, and refuses the request rather than silently dropping further data if the conservative 48 KB request ceiling is still exceeded. This leaves margin for TypeSafe's token-based limit because telemetry JSON tokenizes more densely than prose. The exact request is visible in **Raw data**.

The request uses the current official TypeSafe HTTP API:

- `POST https://api.typesafe.ai/v1/systemone`
- model `jev-latest`
- one `Score` for the ten slowdown-severity levels
- one `Choice` for the competing primary process explanations
- one independent `Noul` per relevant process
- one conditional mechanism `Choice` per relevant process

Questions run independently over the same state. Contribution probabilities intentionally do not sum to 100%; the primary-process Choice distribution does.

## Telemetry and privileges

The application itself must not be run as root. Its active collectors require no elevation:

- libproc `proc_listallpids`, `proc_pidinfo`, `proc_pidpath`, and `proc_pid_rusage`
- `RUSAGE_INFO_CURRENT`, with runtime fallback to the V5/V4 prefix only if the running OS rejects the SDK-current flavor
- Mach host CPU and VM statistics
- read-only `sysctlbyname` reads for physical memory and swap usage
- `getifaddrs` interface counters
- fixed read-only `ioreg -a -r -c` plist queries for AGX and storage registry properties

`powermetrics --help` is queried at runtime to detect supported structured output. Sampling with powermetrics remains unavailable because this build does not install a privileged helper or prompt repeatedly for administrator access. Consequently system GPU/ANE power, process network accounting, wait/SFI data, and exact power limits are shown as unavailable. This is a deliberate safety boundary, not a zero reading.

Per-process GPU time is attempted through the undocumented AGX IORegistry `AppUsage.accumulatedGPUTime` observation path. It is private, capability-detected, read-only, and expected to change across macOS releases.

Per-process ANE execution is not reliably observable. Process neural footprint from current rusage is memory accounting and is not ANE utilization. Power-limit fields are hardware/macOS-dependent. Some process fields can be missing because EPERM, ESRCH, spawn, exit, and PID reuse are ordinary sampling conditions.

See [telemetry-sources.md](docs/telemetry-sources.md) for units, derivation, API status, and dependencies.

## Read-only guarantee

There is no frontend command executor, privileged shell bridge, debugger attachment, `task_for_pid`, process control call, IORegistry write, sysctl write, device write, scheduler control, power control, GPU/ANE control, system extension, or configuration mutation.

The only subprocesses have fixed executable paths and fixed observational arguments. Event JSON is the application’s own output and is written only to its Tauri app-data `events` directory. See [read-only-audit.md](docs/read-only-audit.md) for the native-call audit.

## Local history and raw inspection

Completed and failed diagnostic events are stored as local JSON under the application data directory, normally beneath:

```text
~/Library/Application Support/com.omavashia.jev-mac-doctor/events
```

The **Raw data** tab exposes retained system/process samples, capability state, the exact compacted Jev state, question definitions, raw response, normalized response, and errors. Each section can be copied as JSON.

## Tests and checks

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run lint
npm run typecheck
npm test
npm run build
npm run tauri build
```

Native tests cover stable process identity, PID reuse, current-rusage field mapping, delta reset/wrap handling, AGX time parsing, powermetrics plist parsing, missing optional telemetry, runtime capability construction, rolling-window extraction, capture state construction, Jev serialization/normalization, persistence, and collector error isolation. Hardware-dependent inputs use isolated in-memory plist fixtures; production never falls back to mock telemetry.

## Known limitations

- A safe privileged powermetrics helper is not implemented, so its dependent metrics remain unavailable.
- AGX per-process GPU telemetry is private and may be absent or change schema.
- Bundle identifiers, responsible PID, coalition ID, per-process network, and wait/SFI time are optional and currently unavailable without a supported source.
- Interface byte counters exposed through `getifaddrs` are 32-bit on macOS. The rate logic handles a near-maximum wrap and rejects ordinary resets.
- System thermal state is not populated until a supported public binding or safe powermetrics source is available.
- Jev accepts structured text state, not every raw sample from every process. The full local event is preserved and the sent subset is explicit.
