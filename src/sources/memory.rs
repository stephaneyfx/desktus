// Copyright (C) 2019-2024 Stephane Raux. Distributed under the 0BSD license.

use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemoryUsage {
    pub used: u64,
    pub total: u64,
}

pub fn memory_usage(system: &mut System) -> MemoryUsage {
    system.refresh_memory();
    MemoryUsage {
        used: system.used_memory(),
        total: system.total_memory(),
    }
}
