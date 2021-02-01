// Copyright (C) 2019 Stephane Raux. Distributed under the MIT license.

#![deny(warnings)]

use async_ctrlc::CtrlC;
use async_std::prelude::FutureExt as _;
use chrono::Local;
use futures::{Stream, StreamExt, TryFutureExt};
use palette::Srgb;
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
    color: Option<Srgb<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_color")]
    background: Option<Srgb<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_color")]
    border: Option<Srgb<u8>>,
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

    fn critical(self) -> Block {
        Block { background: Some(CRITICAL_COLOR), ..self }
    }

    fn critical_if(self, yes: bool) -> Block {
        if yes { self.critical() } else { self }
    }
}

fn serialize_color<S>(color: &Option<Srgb<u8>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = color.as_ref().map(|c| format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue))
        .unwrap_or(String::new());
    serializer.serialize_str(&s)
}

async fn write_blocks() {
    let streams = vec![
        render_disk_usage().boxed(),
        render_memory_usage().boxed(),
        render_cpu_usage().boxed(),
        render_battery().boxed(),
        render_time().boxed(),
    ];
    let streams = streams.into_iter().enumerate()
        .map(|(index, blocks)| blocks.map(move |block| (index, block)));
    let mut blocks = vec![Vec::new(); streams.len()];
    let mut block_stream = futures::stream::select_all(streams);
    let mut refresh_time = Instant::now();
    let refresh_period = Duration::from_secs(1);
    while let Some((index, new_blocks)) = block_stream.next().await {
        blocks[index] = new_blocks;
        let t = Instant::now();
        if t - refresh_time >= refresh_period {
            refresh_time = t;
            render_blocks(&blocks);
        }
    }
}

fn render_blocks(blocks: &[Vec<Block>]) {
    let blocks = blocks.iter().flatten().collect::<Vec<_>>();
    let line = serde_json::to_string(&blocks).unwrap();
    println!("{},", line);
}

fn render_time() -> impl Stream<Item = Vec<Block>> {
    async_std::stream::once(())
        .chain(async_std::stream::interval(Duration::from_secs(1)))
        .map(|_| {
            let t = Local::now();
            vec![
                Block::new(t.format("%a %d %b %Y").to_string()),
                Block::new(t.format("%R").to_string()),
            ]
        })
}

fn render_battery() -> impl Stream<Item = Vec<Block>> {
    async_std::stream::once(())
        .chain(async_std::stream::interval(Duration::from_secs(60)))
        .map(|_| make_battery_block().ok().flatten().map_or(Vec::new(), |b| vec![b]))
}

fn make_battery_block() -> Result<Option<Block>, battery::Error> {
    let manager = battery::Manager::new()?;
    if manager.batteries()?.next().is_none() {
        return Ok(None)
    }
    let (battery_count, capacity, charging) = manager.batteries()?
        .try_fold((0, 0.0, false), |(battery_count, capacity, charging), battery| {
            let battery = battery?;
            let battery_count = battery_count + 1;
            let capacity = capacity + battery.state_of_charge().value;
            let battery_state = battery.state();
            let charging = charging
                || battery_state == battery::State::Charging;
            Ok::<_, battery::Error>((battery_count, capacity, charging))
        })?;
    if battery_count == 0 { return Ok(None); }
    let capacity = capacity / battery_count as f32;
    let icon = battery_icon(capacity, charging);
    let capacity = capacity * 100.0;
    let capacity = capacity.round() as u32;
    let block = Block::new(format!("{} {}%", icon, capacity));
    Ok(Some(block.critical_if(capacity <= 10)))
}

fn battery_icon(capacity: f32, charging: bool) -> char {
    const ICONS: &[[char; 2]] = &[
        ['\u{f008e}', '\u{f089f}'],
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
    let capacity = ((capacity * ICONS.len() as f32).floor() as usize).min(ICONS.len() - 1);
    ICONS[capacity][charging as usize]
}

fn render_cpu_usage() -> impl Stream<Item = Vec<Block>> {
    heim::cpu::time()
        .map_ok(|stats| {
            let stats = CpuTime::from(stats);
            futures::stream::try_unfold(stats, |old_stats| async move {
                let new_stats = CpuTime::from(heim::cpu::time().await?);
                Ok(Some((old_stats.usage(&new_stats), new_stats)))
            })
        })
        .try_flatten_stream()
        .map(|usage| usage.map_or(Vec::new(), |u| vec![cpu_usage_block(u)]))
        .zip(
            async_std::stream::once(())
                .chain(async_std::stream::interval(Duration::from_secs(5)))
        )
        .map(|(u, _)| u)
}

fn cpu_usage_block(usage: u32) -> Block {
    let block = Block::new(format!("\u{f061a} {}%", usage));
    block.critical_if(usage >= 95)
}

#[derive(Debug)]
struct CpuTime {
    user: Duration,
    system: Duration,
    idle: Duration,
}

impl CpuTime {
    fn usage(&self, end: &Self) -> u32 {
        let user_elapsed = end.user - self.user;
        let system_elapsed = end.system - self.system;
        let idle_elapsed = end.idle - self.idle;
        let busy_elapsed = user_elapsed + system_elapsed;
        let total_elapsed = busy_elapsed + idle_elapsed;
        if total_elapsed == Duration::default() { return 0; }
        (busy_elapsed.as_secs_f64() / total_elapsed.as_secs_f64() * 100.0) as u32
    }
}

impl From<heim::cpu::CpuTime> for CpuTime {
    fn from(reading: heim::cpu::CpuTime) -> Self {
        let user = duration(reading.user());
        let system = duration(reading.system());
        let idle = duration(reading.idle());
        Self { user, system, idle }
    }
}

fn render_memory_usage() -> impl Stream<Item = Vec<Block>> {
    async_std::stream::once(())
        .chain(async_std::stream::interval(Duration::from_secs(30)))
        .then(|_| async {
            let mem_stats = heim::memory::memory().await.ok()?;
            Some(memory_usage_block(mem_stats))
        })
        .map(|usage| usage.map_or(Vec::new(), |u| vec![u]))
}

fn memory_usage_block(mem_stats: heim::memory::Memory) -> Block {
    let total = gibibytes(mem_stats.total());
    let available = gibibytes(mem_stats.available());
    let used = total - available;
    let block = Block::new(format!("\u{f035b} {:.2} / {:.2} GiB", used, total));
    block.critical_if(used / total >= 0.95)
}

fn render_disk_usage() -> impl Stream<Item = Vec<Block>> {
    async_std::stream::once(())
        .chain(async_std::stream::interval(Duration::from_secs(120)))
        .then(|_| async {
            let (used, total) = heim::disk::partitions_physical()
                .filter_map(|partition| async move {
                    let usage = heim::disk::usage(partition.ok()?.mount_point()).await.ok()?;
                    Some((mebibytes_nat(usage.used()), mebibytes_nat(usage.total())))
                })
                .fold((0u64, 0u64), |(acc_used, acc_total), (used, total)| async move {
                    (acc_used + used, acc_total + total)
                })
                .await;
            vec![disk_usage_block(used as f64 / 1000.0, total as f64 / 1000.0)]
        })
}

fn disk_usage_block(used: f64, total: f64) -> Block {
    Block::new(format!("\u{f01bc} {:.2} / {:.2} GiB", used, total))
        .critical_if(used / total >= 0.9)
}

fn duration(t: heim::units::Time) -> Duration {
    Duration::from_nanos(t.get::<heim::units::time::nanosecond>() as u64)
}

fn mebibytes_nat(q: heim::units::Information) -> u64 {
    q.get::<heim::units::information::mebibyte>()
}

fn gibibytes(q: heim::units::Information) -> f64 {
    q.get::<heim::units::information::mebibyte>() as f64 / 1000.0
}

const CRITICAL_COLOR: Srgb<u8> = palette::named::FIREBRICK;
