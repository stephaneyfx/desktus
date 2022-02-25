// Copyright (C) 2022 Stephane Raux. Distributed under the 0BSD license.

use brightness::Brightness;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BrightnessInfo {
    pub device: String,
    pub level: u32,
}

pub async fn brightness() -> Vec<Result<BrightnessInfo, BrightnessError>> {
    brightness::brightness_devices()
        .then(|device| async move {
            let device = device?;
            let name = device.device_name().await?;
            let level = device.get().await?;
            Ok(BrightnessInfo {
                device: name,
                level,
            })
        })
        .map_err(BrightnessError)
        .collect()
        .await
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct BrightnessError(brightness::Error);
