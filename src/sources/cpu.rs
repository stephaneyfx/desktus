// Copyright (C) 2019-2024 Stephane Raux. Distributed under the 0BSD license.

use sysinfo::{CpuExt, System, SystemExt};

pub fn cpu_usage(system: &mut System) -> u32 {
    system.refresh_cpu();
    let usage = system.global_cpu_info().cpu_usage();
    100.min(usage.round() as u32)
}
