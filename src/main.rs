// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

#![deny(warnings)]

use brightness::{brightness_devices, Brightness};
use chrono::Local;
use futures::{pin_mut, FutureExt, Stream, StreamExt};
use number_prefix::NumberPrefix;
use palette::Srgb;
use serde::{Serialize, Serializer};
use std::{
    collections::HashSet,
    fmt::{self, Display},
    ops::{Add, AddAssign, Sub},
    time::{Duration, Instant},
};
use sysinfo::{DiskExt, NetworkExt, ProcessorExt, System, SystemExt};

#[tokio::main]
async fn main() {
    println!(r#"{{ "version": 1 }}"#);
    println!("[");
    let ctrl_c = tokio::signal::ctrl_c();
    pin_mut!(ctrl_c);
    futures::future::select(write_blocks().boxed(), ctrl_c).await;
    println!("[]");
    println!("]");
}

#[derive(Clone, Debug, Serialize)]
enum BlockName {
    Date,
    Time,
    Battery,
    CpuUsage,
    MemoryUsage,
    DiskUsage,
    NetReadSpeed,
    NetWriteSpeed,
    Brightness,
}

impl BlockName {
    fn build<S: Into<String>>(self, text: S) -> Block {
        Block::new(self, text)
    }
}

#[derive(Clone, Debug, Serialize)]
struct Block {
    name: BlockName,
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,
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
    fn new<S: Into<String>>(name: BlockName, text: S) -> Block {
        Block {
            name,
            instance: None,
            full_text: text.into(),
            short_text: None,
            color: None,
            background: None,
            border: None,
            urgent: false,
            separator: true,
        }
    }

    fn critical(self) -> Block {
        Block {
            background: Some(CRITICAL_COLOR),
            ..self
        }
    }

    fn critical_if(self, yes: bool) -> Block {
        if yes {
            self.critical()
        } else {
            self
        }
    }

    fn with_instance<S: Into<String>>(self, instance: S) -> Self {
        Self {
            instance: Some(instance.into()),
            ..self
        }
    }
}

fn serialize_color<S>(color: &Option<Srgb<u8>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = color
        .as_ref()
        .map(|c| format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue))
        .unwrap_or(String::new());
    serializer.serialize_str(&s)
}

async fn write_blocks() {
    let streams = vec![
        render_brightness().boxed(),
        render_nic_info().boxed(),
        render_disk_usage().boxed(),
        render_memory_usage().boxed(),
        render_cpu_usage().boxed(),
        render_battery().boxed(),
        render_time().boxed(),
    ];
    let streams = streams
        .into_iter()
        .enumerate()
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
    let blocks = || {
        let t = Local::now();
        vec![
            BlockName::Date.build(t.format("%a %d %b %Y").to_string()),
            BlockName::Time.build(t.format("%R").to_string()),
        ]
    };
    throttle(Duration::from_secs(1), futures::stream::repeat_with(blocks))
}

fn render_battery() -> impl Stream<Item = Vec<Block>> {
    let blocks = || make_battery_block().ok().flatten().into_iter().collect();
    throttle(
        Duration::from_secs(60),
        futures::stream::repeat_with(blocks),
    )
}

fn make_battery_block() -> Result<Option<Block>, battery::Error> {
    let manager = battery::Manager::new()?;
    let (battery_count, capacity, charging) = manager.batteries()?.try_fold(
        (0, 0.0, false),
        |(battery_count, capacity, charging), battery| {
            let battery = battery?;
            let battery_count = battery_count + 1;
            let capacity = capacity + battery.state_of_charge().value;
            let battery_state = battery.state();
            let charging = charging || battery_state == battery::State::Charging;
            Ok::<_, battery::Error>((battery_count, capacity, charging))
        },
    )?;
    if battery_count == 0 {
        return Ok(None);
    }
    let capacity = capacity / battery_count as f32;
    let icon = battery_icon(capacity, charging);
    let capacity = capacity * 100.0;
    let capacity = capacity.round() as u32;
    let block = BlockName::Battery.build(format!("{} {}%", icon, capacity));
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
    let mut system = System::new();
    let blocks = futures::stream::repeat_with(move || {
        system.refresh_cpu();
        let x = system.global_processor_info().cpu_usage();
        vec![cpu_usage_block(x as u32)]
    });
    throttle(Duration::from_secs(5), blocks)
}

fn cpu_usage_block(usage: u32) -> Block {
    let block = BlockName::CpuUsage.build(format!("\u{f08d6} {}%", usage));
    block.critical_if(usage >= 95)
}

fn render_memory_usage() -> impl Stream<Item = Vec<Block>> {
    let mut system = System::new();
    let blocks = futures::stream::repeat_with(move || {
        system.refresh_memory();
        vec![memory_usage_block(
            system.used_memory(),
            system.total_memory(),
        )]
    });
    throttle(Duration::from_secs(5), blocks)
}

fn memory_usage_block(used: u64, total: u64) -> Block {
    let (used, total) = ((used * 1000) as f64, (total * 1000) as f64);
    let block = BlockName::MemoryUsage.build(format!(
        "\u{f1296} {} / {}",
        pretty_bytes(used),
        pretty_bytes(total)
    ));
    block.critical_if(used / total >= 0.95)
}

fn render_disk_usage() -> impl Stream<Item = Vec<Block>> {
    let mut system = System::new();
    let blocks = futures::stream::repeat_with(move || {
        system.refresh_disks_list();
        system.refresh_disks();
        let (used, total, _) = system.disks().iter().fold(
            (0, 0, HashSet::new()),
            |(used_acc, total_acc, mut visited), disk| {
                if visited.insert(disk.name().to_owned()) {
                    let total = disk.total_space();
                    (
                        used_acc + total - disk.available_space(),
                        total_acc + total,
                        visited,
                    )
                } else {
                    (used_acc, total_acc, visited)
                }
            },
        );
        vec![disk_usage_block(used as f64, total as f64)]
    });
    throttle(Duration::from_secs(60), blocks)
}

fn disk_usage_block(used: f64, total: f64) -> Block {
    BlockName::DiskUsage
        .build(format!(
            "\u{f01bc} {} / {}",
            pretty_bytes(used),
            pretty_bytes(total)
        ))
        .critical_if(used / total >= 0.9)
}

fn render_nic_info() -> impl Stream<Item = Vec<Block>> {
    let mut system = System::new();
    let mut stats = move || {
        system.refresh_networks_list();
        system.refresh_networks();
        system.networks().into_iter().map(|(_, data)| data).fold(
            IoStats::default(),
            |io_stats, network| {
                io_stats
                    + IoStats {
                        bytes_read: network.received(),
                        bytes_written: network.transmitted(),
                    }
            },
        )
    };
    let _ = stats();
    let blocks = std::iter::successors(Some((Instant::now(), Vec::new())), move |(t, _)| {
        let io_stats = stats();
        let new_t = Instant::now();
        Some((new_t, net_io_blocks(io_stats, new_t - *t)))
    })
    .map(|(_, blocks)| blocks);
    let blocks = futures::stream::iter(blocks);
    throttle(Duration::from_secs(5), blocks)
}

fn net_io_blocks(stats: IoStats, elapsed: Duration) -> Vec<Block> {
    vec![
        bytes_per_second_block(
            BlockName::NetReadSpeed,
            "\u{f01da}",
            stats.bytes_read,
            elapsed,
        ),
        bytes_per_second_block(
            BlockName::NetWriteSpeed,
            "\u{f0552}",
            stats.bytes_written,
            elapsed,
        ),
    ]
}

fn bytes_per_second_block(
    name: BlockName,
    icon: &str,
    byte_count: u64,
    elapsed: Duration,
) -> Block {
    let elapsed = elapsed.as_secs_f64();
    let count = byte_count as f64 / elapsed;
    Block::new(name, format!("{} {}/s", icon, pretty_bytes(count)))
}

fn render_brightness() -> impl Stream<Item = Vec<Block>> {
    let blocks = futures::stream::repeat(()).then(|_| {
        brightness_devices()
            .filter_map(|device| async move {
                let device = device.ok()?;
                let name = device.device_name().await.ok()?;
                let level = device.get().await.ok()?;
                Some(brightness_block(&name, level))
            })
            .collect()
    });
    throttle(Duration::from_secs(5), blocks.boxed())
}

fn brightness_block(device: &str, value: u32) -> Block {
    BlockName::Brightness
        .build(format!("\u{f0379} {}%", value))
        .with_instance(device)
}

#[derive(Clone, Copy, Debug, Default)]
struct IoStats {
    bytes_written: u64,
    bytes_read: u64,
}

impl Add for IoStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            bytes_written: self.bytes_written + rhs.bytes_written,
            bytes_read: self.bytes_read + rhs.bytes_read,
        }
    }
}

impl Sub for IoStats {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            bytes_written: self.bytes_written - rhs.bytes_written,
            bytes_read: self.bytes_read - rhs.bytes_read,
        }
    }
}

impl AddAssign for IoStats {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

#[derive(Clone, Debug)]
struct PrettyQuantity {
    quantity: f64,
}

impl From<f64> for PrettyQuantity {
    fn from(quantity: f64) -> Self {
        Self { quantity }
    }
}

impl Display for PrettyQuantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match NumberPrefix::decimal(self.quantity) {
            NumberPrefix::Standalone(q) => write!(f, "{:.1} ", q),
            NumberPrefix::Prefixed(prefix, q) => write!(f, "{:.1} {}", q, prefix),
        }
    }
}

#[derive(Clone, Debug)]
struct WithUnit {
    value: PrettyQuantity,
    unit: &'static str,
}

impl WithUnit {
    fn new(q: f64, unit: &'static str) -> Self {
        Self {
            value: q.into(),
            unit,
        }
    }
}

impl Display for WithUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.value, self.unit)
    }
}

fn pretty_bytes(n: f64) -> WithUnit {
    WithUnit::new(n, "B")
}

const CRITICAL_COLOR: Srgb<u8> = palette::named::FIREBRICK;

fn throttle<S>(period: Duration, stream: S) -> impl Stream<Item = S::Item>
where
    S: Stream + Unpin,
{
    let interval = tokio::time::interval(period);
    futures::stream::unfold(
        (interval, stream),
        |(mut interval, mut stream)| async move {
            interval.tick().await;
            let item = stream.next().await;
            item.map(|item| (item, (interval, stream)))
        },
    )
}
