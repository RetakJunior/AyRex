#[cfg(windows)]
pub(super) fn sample() -> Option<(u64, u64, f32)> {
    #[repr(C)]
    #[derive(Default)]
    struct MemoryStatus {
        length: u32,
        load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page: u64,
        avail_page: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended: u64,
    }
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatus) -> i32;
    }
    let mut status = MemoryStatus {
        length: std::mem::size_of::<MemoryStatus>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 || status.total_phys == 0 {
        return None;
    }
    Some((
        status.total_phys.saturating_sub(status.avail_phys),
        status.total_phys,
        status.load as f32,
    ))
}

#[cfg(not(windows))]
pub(super) fn sample() -> Option<(u64, u64, f32)> {
    None
}
