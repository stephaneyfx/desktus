// Copyright (C) 2019 Stephane Raux. Distributed under the MIT license.

#![deny(warnings)]

use async_ctrlc::CtrlC;
use async_std::prelude::FutureExt as _;
use chrono::Local;
use futures::{Stream, StreamExt};
use futures::future::ready;
use rgb::RGB8;
use serde::{Serialize, Serializer};
use std::time::{Duration, Instant};

fn main() {
    let ctrlc = CtrlC::new().expect("Failed to register CTRL+C handler");
    println!(r#"{{ "version": 1 }}"#);
    println!("[");
    async_std::task::block_on(write_blocks().race(ctrlc));
    println!("[]");
    println!("]");
}

#[derive(Clone, Debug, Serialize)]
struct Block {
    full_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_color")]
    color: Option<RGB8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_color")]
    background: Option<RGB8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_color")]
    border: Option<RGB8>,
    urgent: bool,
    separator: bool,
}

impl Block {
    fn new<S: Into<String>>(s: S) -> Block {
        Block {
            full_text: s.into(),
            short_text: None,
            color: None,
            background: None,
            border: None,
            urgent: false,
            separator: true,
        }
    }
}

fn serialize_color<S>(color: &Option<RGB8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = color.as_ref().map(|RGB8 { r, g, b }| format!("#{:02x}{:02x}{:02x}", r, g, b))
        .unwrap_or(String::new());
    serializer.serialize_str(&s)
}

async fn write_blocks() {
    let streams = vec![
        render_battery().boxed(),
        render_time().boxed(),
    ];
    let streams = streams.into_iter().enumerate()
        .map(|(index, blocks)| blocks.map(move |block| (index, block)));
    let mut blocks = vec![None; streams.len()];
    let mut block_stream = futures::stream::select_all(streams);
    let mut refresh_time = Instant::now();
    let refresh_period = Duration::from_secs(1);
    while let Some((index, block)) = block_stream.next().await {
        blocks[index] = Some(block);
        let t = Instant::now();
        if t - refresh_time >= refresh_period {
            refresh_time = t;
            render_blocks(&blocks);
        }
    }
}

fn render_blocks(blocks: &[Option<Block>]) {
    let blocks = blocks.iter().cloned().flatten().collect::<Vec<_>>();
    let line = serde_json::to_string(&blocks).unwrap();
    println!("{},", line);
}

fn render_time() -> impl Stream<Item = Block> {
    async_std::stream::once(())
        .chain(async_std::stream::interval(Duration::from_secs(1)))
        .map(|_| {
            let t = Local::now();
            Block::new(t.format("%F %R").to_string())
        })
}

fn render_battery() -> impl Stream<Item = Block> {
    async_std::stream::once(())
        .chain(async_std::stream::interval(Duration::from_secs(60)))
        .filter_map(|_| ready(make_battery_block().ok().flatten()))
}

fn make_battery_block() -> Result<Option<Block>, battery::Error> {
    let manager = battery::Manager::new()?;
    if manager.batteries()?.next().is_none() {
        return Ok(None)
    }
    let (capacity, charging) = manager.batteries()?
        .try_fold((0.0, false), |(capacity, charging), battery| {
            let battery = battery?;
            let capacity = capacity + battery.state_of_charge().value;
            let battery_state = battery.state();
            let charging = charging
                || battery_state == battery::State::Charging;
            Ok::<_, battery::Error>((capacity, charging))
        })?;
    let icon = battery_icon(capacity, charging);
    let capacity = capacity * 100.0;
    let capacity = capacity.round() as u32;
    Ok(Some(Block::new(format!("{} {}%", icon, capacity))))
}

fn battery_icon(capacity: f32, charging: bool) -> char {
    const ICONS: &[[char; 2]] = &[
        ['\u{f007a}', '\u{f089c}'],
        ['\u{f007b}', '\u{f0086}'],
        ['\u{f007c}', '\u{f0087}'],
        ['\u{f007d}', '\u{f0088}'],
        ['\u{f007e}', '\u{f089d}'],
        ['\u{f007f}', '\u{f0089}'],
        ['\u{f0080}', '\u{f089e}'],
        ['\u{f0081}', '\u{f008a}'],
        ['\u{f0082}', '\u{f008b}'],
        ['\u{f0079}', '\u{f0085}'],
    ];
    let capacity = ((capacity * ICONS.len() as f32) as usize).min(ICONS.len() - 1);
    ICONS[capacity][charging as usize]
}
