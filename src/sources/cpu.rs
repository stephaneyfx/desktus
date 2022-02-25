// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

use sysinfo::{ProcessorExt, System, SystemExt};

pub fn cpu_usage() -> u32 {
    let mut system = System::new();
    system.refresh_cpu();
    let usage = system.global_processor_info().cpu_usage();
    100.min(usage.round() as u32)
}
