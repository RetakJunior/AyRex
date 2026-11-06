mod cpu;
mod memory;

use std::time::{Duration, Instant};

#[derive(Default)]
pub struct PerformanceSampler {
    last_sample: Option<Instant>,
    cpu: cpu::CpuSampler,
    pub cpu_percent: Option<f32>,
    pub ram_percent: Option<f32>,
    pub ram_used: Option<u64>,
    pub ram_total: Option<u64>,
}

impl PerformanceSampler {
    pub fn refresh(&mut self) {
        if self
            .last_sample
            .is_some_and(|last| last.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        self.last_sample = Some(Instant::now());
        self.cpu_percent = self.cpu.sample();
        if let Some((used, total, percent)) = memory::sample() {
            self.ram_used = Some(used);
            self.ram_total = Some(total);
            self.ram_percent = Some(percent);
        }
    }
}
