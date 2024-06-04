// Copyright (C) 2019-2024 Stephane Raux. Distributed under the 0BSD license.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BatteryState {
    pub capacity: u32,
    pub charging: bool,
}

pub fn battery_state() -> Result<Option<BatteryState>, battery::Error> {
    let manager = battery::Manager::new()?;
    let (count, capacity, charging) = manager.batteries()?.try_fold(
        (0, 0.0, false),
        |(count, capacity, charging), battery| {
            let battery = battery?;
            let count = count + 1;
            let capacity = capacity + battery.state_of_charge().value;
            let charging = charging || battery.state() == battery::State::Charging;
            Ok::<_, battery::Error>((count, capacity, charging))
        },
    )?;
    if count == 0 {
        return Ok(None);
    }
    let capacity = capacity / count as f32 * 100.0;
    let capacity = 100.min(capacity.round() as u32);
    Ok(Some(BatteryState { capacity, charging }))
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct BatteryError(battery::Error);
