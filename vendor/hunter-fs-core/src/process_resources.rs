//! OS snapshots for this process only. No child process, external tool or user-tree read.
#[derive(Clone, Copy, Debug, Default)]
pub struct Reading {
    /// Cumulative user and system CPU nanoseconds, independently unavailable on failure.
    pub cpu: Option<(u64, u64)>,
    /// Current resident bytes (Windows working set), not peak RSS or physical footprint.
    pub rss: Option<u64>,
}

/// Read independent best-effort process counters; missing data stays `None`, never zero.
/// Unsupported platforms return an empty reading. Samples are not atomic or per-scan.
pub fn read() -> Reading {
    platform::read()
}

#[cfg(all(
    target_pointer_width = "64",
    any(target_os = "macos", target_os = "linux")
))]
mod unix_cpu {
    #[derive(Default)]
    #[repr(C)]
    struct Timeval {
        seconds: i64,
        #[cfg(target_os = "macos")]
        microseconds: i32,
        #[cfg(target_os = "linux")]
        microseconds: i64,
    }
    #[derive(Default)]
    #[repr(C)]
    struct Usage {
        user: Timeval,
        system: Timeval,
        counters: [i64; 14],
    }
    unsafe extern "C" {
        fn getrusage(who: i32, usage: *mut Usage) -> i32;
    }
    fn ns(value: Timeval) -> Option<u64> {
        let seconds = u64::try_from(value.seconds).ok()?;
        let micros = u64::try_from(value.microseconds)
            .ok()
            .filter(|value| *value < 1_000_000)?;
        seconds
            .checked_mul(1_000_000_000)?
            .checked_add(micros * 1000)
    }
    pub(super) fn read() -> Option<(u64, u64)> {
        const _: () = assert!(size_of::<Usage>() == 144);
        let mut usage = Usage::default();
        // SAFETY: RUSAGE_SELF=0 and the complete platform 64-bit rusage layout.
        (unsafe { getrusage(0, &mut usage) } == 0).then_some(())?;
        Some((ns(usage.user)?, ns(usage.system)?))
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn conversion_rejects_invalid_or_overflowing_timevals() {
            assert_eq!(
                ns(Timeval {
                    seconds: 2,
                    microseconds: 3
                }),
                Some(2_000_003_000)
            );
            for (seconds, microseconds) in [(-1, 0), (0, -1), (0, 1_000_000), (i64::MAX, 0)] {
                assert_eq!(
                    ns(Timeval {
                        seconds,
                        microseconds
                    }),
                    None
                );
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{Reading, unix_cpu};
    #[derive(Default)]
    #[repr(C)]
    struct UsageInfo {
        uuid: [u8; 16],
        user: u64,
        system: u64,
        idle: u64,
        interrupts: u64,
        pageins: u64,
        wired: u64,
        resident: u64,
        footprint: u64,
        start: u64,
        exit: u64,
    }
    #[link(name = "proc")]
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut std::ffi::c_void) -> i32;
    }
    pub(super) fn read() -> Reading {
        const _: () = assert!(size_of::<UsageInfo>() == 96);
        let mut info = UsageInfo::default();
        // SAFETY: public RUSAGE_INFO_V0=0 and its complete SDK-defined 96-byte layout.
        // libproc's rusage_info_t* parameter receives the struct buffer itself.
        let success = unsafe {
            proc_pid_rusage(
                std::process::id() as i32,
                0,
                (&mut info as *mut UsageInfo).cast(),
            )
        } == 0;
        Reading {
            cpu: unix_cpu::read(),
            rss: success.then_some(info.resident),
        }
    }
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
mod platform {
    use super::{Reading, unix_cpu};
    unsafe extern "C" {
        fn getpagesize() -> i32;
    }
    pub(super) fn read() -> Reading {
        // procfs is kernel accounting, not a read of the selected scan tree.
        let rss = std::fs::read_to_string("/proc/self/statm")
            .ok()
            .and_then(|text| {
                let pages = text.split_whitespace().nth(1)?.parse::<u64>().ok()?;
                // SAFETY: getpagesize has no arguments and returns this process's page size.
                let page_size = u64::try_from(unsafe { getpagesize() })
                    .ok()
                    .filter(|size| *size > 0)?;
                pages.checked_mul(page_size)
            });
        Reading {
            cpu: unix_cpu::read(),
            rss,
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::Reading;
    use std::ffi::c_void;
    #[derive(Default)]
    #[repr(C)]
    struct Filetime {
        low: u32,
        high: u32,
    }
    impl Filetime {
        fn ns(self) -> Option<u64> {
            ((u64::from(self.high) << 32) | u64::from(self.low)).checked_mul(100)
        }
    }
    #[derive(Default)]
    #[repr(C)]
    struct MemoryCounters {
        size: u32,
        faults: u32,
        peak_rss: usize,
        rss: usize,
        peak_paged: usize,
        paged: usize,
        peak_nonpaged: usize,
        nonpaged: usize,
        pagefile: usize,
        peak_pagefile: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessTimes(
            process: *mut c_void,
            creation: *mut Filetime,
            exit: *mut Filetime,
            kernel: *mut Filetime,
            user: *mut Filetime,
        ) -> i32;
        fn K32GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut MemoryCounters,
            size: u32,
        ) -> i32;
    }
    pub(super) fn read() -> Reading {
        let (mut creation, mut exit, mut kernel, mut user) = (
            Filetime::default(),
            Filetime::default(),
            Filetime::default(),
            Filetime::default(),
        );
        let mut memory = MemoryCounters {
            size: size_of::<MemoryCounters>() as u32,
            ..Default::default()
        };
        // SAFETY: pseudo-handle for self; all output buffers have their full Win32 layouts.
        unsafe {
            let process = GetCurrentProcess();
            let cpu = (GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user)
                != 0)
                .then(|| Some((user.ns()?, kernel.ns()?)))
                .flatten();
            let size = memory.size;
            let rss = (K32GetProcessMemoryInfo(process, &mut memory, size) != 0)
                .then_some(memory.rss as u64);
            Reading { cpu, rss }
        }
    }
}

#[cfg(not(any(
    target_os = "macos",
    windows,
    all(target_os = "linux", target_pointer_width = "64")
)))]
mod platform {
    pub(super) fn read() -> super::Reading {
        super::Reading::default()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn absent_counters_are_not_fabricated_zeroes() {
        let reading = super::Reading::default();
        assert_eq!(reading.cpu, None);
        assert_eq!(reading.rss, None);
    }
    #[cfg(not(any(
        target_os = "macos",
        windows,
        all(target_os = "linux", target_pointer_width = "64")
    )))]
    #[test]
    fn unsupported_platform_has_no_counters() {
        let reading = super::read();
        assert_eq!(reading.cpu, None);
        assert_eq!(reading.rss, None);
    }
    #[cfg(any(
        target_os = "macos",
        windows,
        all(target_os = "linux", target_pointer_width = "64")
    ))]
    #[test]
    fn current_process_has_resident_memory_and_monotonic_cpu_accounting() {
        let before = super::read();
        let after = super::read();
        assert!(before.rss.is_some_and(|bytes| bytes > 0));
        let (u0, s0) = before.cpu.expect("process CPU reading");
        let (u1, s1) = after.cpu.expect("second process CPU reading");
        assert!(u1 >= u0 && s1 >= s0);
    }
}
