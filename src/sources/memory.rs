// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemoryUsage {
    pub used: u64,
    pub total: u64,
}

pub fn memory_usage() -> MemoryUsage {
    let mut system = System::new();
    system.refresh_memory();
    MemoryUsage {
        used: system.used_memory() * 1000,
        total: system.total_memory() * 1000,
    }
}
