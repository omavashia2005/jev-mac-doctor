use std::{
    collections::HashSet,
    ffi::{CStr, CString},
    mem::{size_of, MaybeUninit},
    ptr,
};

#[allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
pub(crate) mod ffi {
    include!(concat!(env!("OUT_DIR"), "/macos.rs"));
}

#[derive(Clone, Debug)]
pub struct NativeProcess {
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,
    pub start_seconds: u64,
    pub start_microseconds: u64,
    pub name: String,
    pub executable: Option<String>,
    pub usage: ffi::rusage_info_v6,
    pub rusage_version: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CpuTicks {
    pub user: u64,
    pub system: u64,
    pub idle: u64,
    pub nice: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VmCounters {
    pub physical_bytes: u64,
    pub free_bytes: u64,
    pub active_bytes: u64,
    pub inactive_bytes: u64,
    pub wired_bytes: u64,
    pub compressed_bytes: u64,
    pub swap_used_bytes: u64,
    pub pageins: u64,
    pub pageouts: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NetworkCounters {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

pub fn processes() -> Result<Vec<NativeProcess>, String> {
    // SAFETY: proc_listallpids is a read-only libproc query. The null first call
    // asks for capacity only; the second receives a valid mutable PID buffer of
    // exactly `capacity * sizeof(pid_t)` bytes owned for the duration of the call.
    let count = unsafe { ffi::proc_listallpids(ptr::null_mut(), 0) };
    if count <= 0 {
        return Err(last_os_error("proc_listallpids capacity"));
    }
    let mut pids = vec![0_i32; count as usize + 64];
    let bytes = i32::try_from(pids.len() * size_of::<i32>())
        .map_err(|_| "PID buffer too large".to_owned())?;
    // SAFETY: the pointer is to the live `pids` allocation and `bytes` is its
    // exact byte length. proc_listallpids only fills PID values; it changes no
    // process or system state.
    let returned = unsafe { ffi::proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
    if returned < 0 {
        return Err(last_os_error("proc_listallpids"));
    }
    pids.truncate(returned as usize);
    Ok(pids
        .into_iter()
        .filter(|pid| *pid > 0)
        .filter_map(process)
        .collect())
}

fn process(pid: i32) -> Option<NativeProcess> {
    let mut bsd = MaybeUninit::<ffi::proc_bsdinfo>::zeroed();
    let expected = size_of::<ffi::proc_bsdinfo>();
    // SAFETY: proc_pidinfo with PROC_PIDTBSDINFO is an observational libproc
    // query. `bsd` is a writable buffer of `expected` bytes and is only assumed
    // initialized when the API reports exactly that many bytes. EPERM/ESRCH are
    // ordinary and return None.
    let read = unsafe {
        ffi::proc_pidinfo(
            pid,
            ffi::PROC_PIDTBSDINFO as i32,
            0,
            bsd.as_mut_ptr().cast(),
            expected as i32,
        )
    };
    if read != expected as i32 {
        return None;
    }
    // SAFETY: the exact structure length was validated above.
    let bsd = unsafe { bsd.assume_init() };

    let (usage, rusage_version) = [
        ffi::RUSAGE_INFO_CURRENT,
        ffi::RUSAGE_INFO_V5,
        ffi::RUSAGE_INFO_V4,
    ]
    .into_iter()
    .find_map(|flavor| {
        let mut usage = MaybeUninit::<ffi::rusage_info_v6>::zeroed();
        // SAFETY: proc_pid_rusage only reads accounting counters. Apple declares
        // the buffer as rusage_info_t* (void**) but documents callers casting the
        // address of the output struct itself to that type. We therefore cast the
        // v6-sized SDK-generated buffer address directly; introducing an actual
        // pointer variable here would give the kernel only pointer-sized storage
        // and corrupt the stack. Older runtime flavors write only their documented
        // prefix, leaving later zeroed fields marked unavailable by `rusage_version`.
        let result = unsafe {
            ffi::proc_pid_rusage(
                pid,
                flavor as i32,
                usage.as_mut_ptr().cast::<ffi::rusage_info_t>(),
            )
        };
        if result != 0 {
            return None;
        }
        // SAFETY: success initializes the selected flavor's prefix, and the
        // remaining bytes were initialized to zero before the call.
        Some((unsafe { usage.assume_init() }, flavor))
    })?;

    Some(NativeProcess {
        pid,
        ppid: bsd.pbi_ppid as i32,
        uid: bsd.pbi_uid,
        start_seconds: bsd.pbi_start_tvsec,
        start_microseconds: bsd.pbi_start_tvusec,
        name: c_array_to_string(&bsd.pbi_name).or_else(|| c_array_to_string(&bsd.pbi_comm))?,
        executable: process_path(pid),
        usage,
        rusage_version,
    })
}

fn process_path(pid: i32) -> Option<String> {
    let mut buffer = vec![0_i8; 4096];
    // SAFETY: proc_pidpath is a read-only query. `buffer` is writable for 4096
    // bytes, remains owned here, and the returned length is checked before a
    // CStr view is created.
    let length = unsafe { ffi::proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if length <= 0 || length as usize >= buffer.len() {
        return None;
    }
    // SAFETY: proc_pidpath succeeded and NUL-terminates within the validated
    // buffer. The CStr is copied before `buffer` is dropped.
    Some(
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn c_array_to_string<const N: usize>(value: &[i8; N]) -> Option<String> {
    let length = value.iter().position(|byte| *byte == 0).unwrap_or(N);
    (length > 0).then(|| {
        String::from_utf8_lossy(
            &value[..length]
                .iter()
                .map(|byte| *byte as u8)
                .collect::<Vec<_>>(),
        )
        .into_owned()
    })
}

pub fn cpu_ticks() -> Result<CpuTicks, String> {
    let mut info = MaybeUninit::<ffi::host_cpu_load_info>::zeroed();
    let mut count = (size_of::<ffi::host_cpu_load_info>() / size_of::<i32>()) as u32;
    // SAFETY: host_statistics with HOST_CPU_LOAD_INFO only reads host counters.
    // `info` is the SDK-generated output layout, its pointer is valid for
    // `count` integer slots, and initialization is accepted only on KERN_SUCCESS.
    let result = unsafe {
        ffi::host_statistics(
            ffi::mach_host_self(),
            ffi::HOST_CPU_LOAD_INFO as i32,
            info.as_mut_ptr().cast(),
            &mut count,
        )
    };
    if result != 0 {
        return Err(format!("host_statistics failed with Mach code {result}"));
    }
    // SAFETY: KERN_SUCCESS means the requested host_cpu_load_info was written.
    let info = unsafe { info.assume_init() };
    Ok(CpuTicks {
        user: info.cpu_ticks[ffi::CPU_STATE_USER as usize] as u64,
        system: info.cpu_ticks[ffi::CPU_STATE_SYSTEM as usize] as u64,
        idle: info.cpu_ticks[ffi::CPU_STATE_IDLE as usize] as u64,
        nice: info.cpu_ticks[ffi::CPU_STATE_NICE as usize] as u64,
    })
}

pub fn vm_counters() -> Result<VmCounters, String> {
    let host = unsafe {
        // SAFETY: mach_host_self returns a send right for read-only host queries;
        // no host state is modified.
        ffi::mach_host_self()
    };
    let mut page_size = 0_usize;
    // SAFETY: host_page_size only reads the VM page size and writes one u32 to
    // the valid `page_size` reference.
    let page_result = unsafe { ffi::host_page_size(host, &mut page_size) };
    if page_result != 0 {
        return Err(format!(
            "host_page_size failed with Mach code {page_result}"
        ));
    }
    let mut vm = MaybeUninit::<ffi::vm_statistics64>::zeroed();
    let mut count = (size_of::<ffi::vm_statistics64>() / size_of::<i32>()) as u32;
    // SAFETY: host_statistics64 with HOST_VM_INFO64 is observational. `vm` has
    // the SDK-generated layout and sufficient integer slots; no pointer escapes.
    let result = unsafe {
        ffi::host_statistics64(
            host,
            ffi::HOST_VM_INFO64 as i32,
            vm.as_mut_ptr().cast(),
            &mut count,
        )
    };
    if result != 0 {
        return Err(format!("host_statistics64 failed with Mach code {result}"));
    }
    // SAFETY: KERN_SUCCESS means the vm_statistics64 buffer was initialized.
    let vm = unsafe { vm.assume_init() };
    let page = page_size as u64;
    Ok(VmCounters {
        physical_bytes: sysctl_u64("hw.memsize").unwrap_or_default(),
        free_bytes: (vm.free_count as u64 + vm.speculative_count as u64) * page,
        active_bytes: vm.active_count as u64 * page,
        inactive_bytes: vm.inactive_count as u64 * page,
        wired_bytes: vm.wire_count as u64 * page,
        compressed_bytes: vm.compressor_page_count as u64 * page,
        swap_used_bytes: swap_used().unwrap_or_default(),
        pageins: vm.pageins,
        pageouts: vm.pageouts,
    })
}

fn sysctl_u64(name: &str) -> Option<u64> {
    let name = CString::new(name).ok()?;
    let mut value = 0_u64;
    let mut size = size_of::<u64>();
    // SAFETY: sysctlbyname is called with newp=null/0, so it is strictly a read.
    // `value` is a valid `size`-byte output buffer and the C name lives through
    // the call. No kernel setting can be changed by this argument shape.
    let result = unsafe {
        ffi::sysctlbyname(
            name.as_ptr(),
            (&mut value as *mut u64).cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };
    (result == 0 && size == size_of::<u64>()).then_some(value)
}

fn swap_used() -> Option<u64> {
    let name = CString::new("vm.swapusage").ok()?;
    let mut value = MaybeUninit::<ffi::xsw_usage>::zeroed();
    let mut size = size_of::<ffi::xsw_usage>();
    // SAFETY: this sysctlbyname invocation is read-only because newp is null and
    // newlen is zero. The SDK-generated xsw_usage output buffer is valid for the
    // advertised length and is interpreted only after length validation.
    let result = unsafe {
        ffi::sysctlbyname(
            name.as_ptr(),
            value.as_mut_ptr().cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };
    if result != 0 || size != size_of::<ffi::xsw_usage>() {
        return None;
    }
    // SAFETY: the successful sysctl read returned the complete structure.
    Some(unsafe { value.assume_init() }.xsu_used)
}

pub fn network_counters() -> Result<NetworkCounters, String> {
    let mut head: *mut ffi::ifaddrs = ptr::null_mut();
    // SAFETY: getifaddrs allocates a read-only snapshot list and writes its head
    // pointer. Ownership is released exactly once with freeifaddrs below.
    if unsafe { ffi::getifaddrs(&mut head) } != 0 {
        return Err(last_os_error("getifaddrs"));
    }
    let mut current = head;
    let mut seen = HashSet::new();
    let mut totals = NetworkCounters::default();
    while !current.is_null() {
        // SAFETY: nodes belong to the getifaddrs list and remain valid until the
        // matching freeifaddrs call after iteration.
        let item = unsafe { &*current };
        if !item.ifa_addr.is_null()
            && item.ifa_flags & ffi::IFF_LOOPBACK == 0
            // SAFETY: ifa_addr is non-null and points to a sockaddr owned by the
            // list; reading its family byte is within that structure.
            && unsafe { (*item.ifa_addr).sa_family as i32 } == ffi::AF_LINK as i32
            && !item.ifa_data.is_null()
        {
            // SAFETY: a non-null name in a valid list node is NUL-terminated and
            // stays live until freeifaddrs.
            let name = unsafe { CStr::from_ptr(item.ifa_name) }
                .to_string_lossy()
                .into_owned();
            if seen.insert(name) {
                // SAFETY: for AF_LINK entries macOS documents ifa_data as an
                // `if_data` snapshot. We only read counters while the list lives.
                let data = unsafe { &*(item.ifa_data.cast::<ffi::if_data>()) };
                totals.rx_bytes = totals.rx_bytes.saturating_add(data.ifi_ibytes as u64);
                totals.tx_bytes = totals.tx_bytes.saturating_add(data.ifi_obytes as u64);
                totals.rx_packets = totals.rx_packets.saturating_add(data.ifi_ipackets as u64);
                totals.tx_packets = totals.tx_packets.saturating_add(data.ifi_opackets as u64);
            }
        }
        current = item.ifa_next;
    }
    // SAFETY: `head` was returned by getifaddrs and has not been freed or
    // retained elsewhere. freeifaddrs only releases this snapshot allocation.
    unsafe { ffi::freeifaddrs(head) };
    Ok(totals)
}

fn last_os_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}
