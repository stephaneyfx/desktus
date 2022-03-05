// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

use serde::{Deserialize, Serialize};
use std::{io, path::Path};
use sysinfo::{DiskExt, System, SystemExt};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiskUsage {
    pub used: u64,
    pub total: u64,
}

pub fn disk_usage<P>(system: &mut System, mount_point: P) -> Result<DiskUsage, io::Error>
where
    P: AsRef<Path>,
{
    let mount_point = mount_point.as_ref();
    system.refresh_disks_list();
    system.refresh_disks();
    system
        .disks()
        .iter()
        .find(|disk| disk.mount_point() == mount_point)
        .map(|disk| {
            let total = disk.total_space();
            let used = total - disk.available_space();
            DiskUsage { used, total }
        })
        .map_or_else(|| Err(io::ErrorKind::NotFound.into()), Ok)
}
