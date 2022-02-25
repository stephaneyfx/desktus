// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

// #![deny(warnings)]

use chrono::Local;
use desktus::ticks;
use futures::StreamExt;
use futuristic::StreamTools;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let foreground = palette::named::LIGHTGRAY;
    let date_time = ticks(Duration::from_secs(1)).map(|_| {
        let t = Local::now();
        vec![
            desktus::views::DateView::new(t, Message::Ignore, foreground).render(),
            desktus::views::TimeView::new(t, Message::Ignore, foreground).render(),
        ]
    });
    let battery = ticks(Duration::from_secs(20)).map(|_| {
        desktus::sources::battery_state()
            .ok()
            .flatten()
            .map(|b| desktus::views::BatteryView::new(b, Message::Ignore, foreground).render())
    });
    let memory = ticks(Duration::from_secs(5)).map(|_| {
        desktus::views::MemoryView::new(
            desktus::sources::memory_usage(),
            Message::Ignore,
            foreground,
        )
        .render()
    });
    let disk = ticks(Duration::from_secs(20)).map(|_| {
        desktus::sources::disk_usage("/")
            .ok()
            .map(|d| desktus::views::DiskView::new(d, Message::Ignore, foreground).render())
    });
    let brightness = ticks(Duration::from_secs(20)).then(|_| async {
        desktus::sources::brightness()
            .await
            .into_iter()
            .flatten()
            .map(|b| {
                desktus::views::BrightnessView::new(b, |_| Message::Ignore, foreground).render()
            })
            .collect::<Vec<_>>()
    });
    let blocks = brightness
        .zip_latest(disk)
        .zip_latest(memory)
        .zip_latest(battery)
        .zip_latest(date_time)
        .map(|((((brightness, disk), memory), battery), date_time)| {
            brightness
                .into_iter()
                .chain(disk)
                .chain(Some(memory))
                .chain(battery)
                .chain(date_time)
                .collect::<Vec<_>>()
        });
    let output = desktus::serialize_blocks(blocks);
    let input = desktus::events::<Message>().for_each(|_| async {});
    futures::future::join(output, input).await;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum Message {
    Ignore,
}
