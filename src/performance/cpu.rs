#[derive(Default)]
pub(super) struct CpuSampler {
    previous: Option<(u64, u64)>,
}

impl CpuSampler {
    pub fn sample(&mut self) -> Option<f32> {
        #[cfg(windows)]
        {
            let (idle, total) = system_times()?;
            let current = (idle, total);
            let usage = self.previous.map(|(old_idle, old_total)| {
                let idle_delta = idle.wrapping_sub(old_idle);
                let total_delta = total.wrapping_sub(old_total);
                if total_delta == 0 {
                    0.0
                } else {
                    (100.0 * (total_delta.saturating_sub(idle_delta) as f64 / total_delta as f64))
                        as f32
                }
            });
            self.previous = Some(current);
            return usage;
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

#[cfg(windows)]
fn system_times() -> Option<(u64, u64)> {
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    impl FileTime {
        fn value(&self) -> u64 {
            (u64::from(self.high) << 32) | u64::from(self.low)
        }
    }
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    }
    let (mut idle, mut kernel, mut user) = (
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
    );
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    (ok != 0).then(|| (idle.value(), kernel.value().saturating_add(user.value())))
}

#[cfg(test)]
mod tests {
    #[test]
    fn cpu_delta_math_is_bounded() {
        let idle = 20_u64;
        let total = 100_u64;
        let cpu = (100.0 * (total - idle) as f64 / total as f64) as f32;
        assert!((0.0..=100.0).contains(&cpu));
        assert_eq!(cpu, 80.0);
    }
}
