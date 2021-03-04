// Copyright (C) 2019-2021 Stephane Raux. Distributed under the zlib license.

#![deny(warnings)]

use chrono::Local;
use futures::{
    future::ready,
    FutureExt, pin_mut, Stream, StreamExt,
};
use number_prefix::NumberPrefix;
use palette::Srgb;
use serde::{Serialize, Serializer};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    ops::{Add, AddAssign, Sub},
    time::{Duration, Instant},
};

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
        render_nic_info().boxed(),
        render_disk_io().boxed(),
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
    let blocks = || {
        let t = Local::now();
        vec![
            Block::new(t.format("%a %d %b %Y").to_string()),
            Block::new(t.format("%R").to_string()),
        ]
    };
    throttle(Duration::from_secs(1), futures::stream::repeat_with(blocks))
}

fn render_battery() -> impl Stream<Item = Vec<Block>> {
    let blocks = || make_battery_block().ok().flatten().into_iter().collect();
    throttle(Duration::from_secs(60), futures::stream::repeat_with(blocks))
}

fn make_battery_block() -> Result<Option<Block>, battery::Error> {
    let manager = battery::Manager::new()?;
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
    let blocks = futures::stream::repeat(())
        .then(|_| heim::cpu::time())
        .scan(None::<CpuTime>, |old_stats, new_stats| {
            let new_stats = new_stats.ok().map(CpuTime::from);
            let blocks = match (&*old_stats, &new_stats) {
                (Some(old), Some(new)) => vec![cpu_usage_block(old.usage(new))],
                _ => Vec::new(),
            };
            *old_stats = new_stats;
            ready(Some(blocks))
        });
    throttle(Duration::from_secs(5), blocks)
}

fn cpu_usage_block(usage: u32) -> Block {
    let block = Block::new(format!("\u{f08d6} {}%", usage));
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
    let blocks = futures::stream::repeat(())
        .then(|_| async {
            heim::memory::memory()
                .await
                .ok()
                .map(memory_usage_block)
                .into_iter()
                .collect()
        })
        .boxed();
    throttle(Duration::from_secs(5), blocks)
}

fn memory_usage_block(mem_stats: heim::memory::Memory) -> Block {
    let total = bytes(mem_stats.total());
    let available = bytes(mem_stats.available());
    let used = total - available;
    let (used, total) = (used as f64, total as f64);
    let block = Block::new(format!("\u{f1296} {} / {}", pretty_bytes(used), pretty_bytes(total)));
    block.critical_if(used / total >= 0.95)
}

fn render_disk_usage() -> impl Stream<Item = Vec<Block>> {
    let blocks = futures::stream::repeat(())
        .then(|_| async {
            let (used, total) = heim::disk::partitions_physical()
                .filter_map(|partition| async move {
                    let usage = heim::disk::usage(partition.ok()?.mount_point()).await.ok()?;
                    Some((bytes(usage.used()), bytes(usage.total())))
                })
                .fold((0u64, 0u64), |(acc_used, acc_total), (used, total)| async move {
                    (acc_used + used, acc_total + total)
                })
                .await;
            vec![disk_usage_block(used as f64, total as f64)]
        })
        .boxed();
    throttle(Duration::from_secs(60), blocks)
}

fn disk_usage_block(used: f64, total: f64) -> Block {
    Block::new(format!("\u{f01bc} {} / {}", pretty_bytes(used), pretty_bytes(total)))
        .critical_if(used / total >= 0.9)
}

fn render_nic_info() -> impl Stream<Item = Vec<Block>> {
    let blocks = NetIoSnapshot::capture()
        .map(|snapshot| futures::stream::unfold(snapshot, |old_snapshot| async move {
            let new_snapshot = NetIoSnapshot::capture().await;
            let diff = new_snapshot.stats - old_snapshot.stats;
            let elapsed = new_snapshot.time - old_snapshot.time;
            Some((net_io_blocks(diff, elapsed), new_snapshot))
        }))
        .flatten_stream()
        .boxed();
    throttle(Duration::from_secs(5), blocks)
}

fn net_io_blocks(stats: NetIoStats, elapsed: Duration) -> Vec<Block> {
    vec![
        bytes_per_second_block("\u{f01da}", stats.external.bytes_read, elapsed),
        bytes_per_second_block("\u{f0552}", stats.external.bytes_written, elapsed),
        bytes_per_second_block("\u{f1320}", stats.loopback.bytes_read, elapsed),
        bytes_per_second_block("\u{f1373}", stats.loopback.bytes_written, elapsed),
    ]
}

fn render_disk_io() -> impl Stream<Item = Vec<Block>> {
    let blocks = DiskIoSnapshot::capture()
        .map(|snapshot| futures::stream::unfold(snapshot, |old_snapshot| async move {
            let new_snapshot = DiskIoSnapshot::capture().await;
            let diff = new_snapshot.stats - old_snapshot.stats;
            let elapsed = new_snapshot.time - old_snapshot.time;
            Some((disk_io_blocks(diff, elapsed), new_snapshot))
        }))
        .flatten_stream()
        .boxed();
    throttle(Duration::from_secs(5), blocks)
}

fn disk_io_blocks(stats: IoStats, elapsed: Duration) -> Vec<Block> {
    vec![
        bytes_per_second_block("\u{f095e}", stats.bytes_read, elapsed),
        bytes_per_second_block("\u{f095d}", stats.bytes_written, elapsed),
    ]
}

fn bytes_per_second_block(icon: &str, byte_count: u64, elapsed: Duration) -> Block {
    let elapsed = elapsed.as_secs_f64();
    let count = byte_count as f64 / elapsed;
    Block::new(format!("{} {}/s", icon, pretty_bytes(count)))
}

#[derive(Clone, Copy, Debug, Default)]
struct IoStats {
    bytes_written: u64,
    bytes_read: u64,
}

impl IoStats {
    fn from_net(counters: &heim::net::IoCounters) -> Self {
        Self {
            bytes_written: bytes(counters.bytes_sent()),
            bytes_read: bytes(counters.bytes_recv()),
        }
    }

    fn from_disk(counters: &heim::disk::IoCounters) -> Self {
        Self {
            bytes_written: bytes(counters.write_bytes()),
            bytes_read: bytes(counters.read_bytes()),
        }
    }
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

#[derive(Clone, Copy, Debug, Default)]
struct NetIoStats {
    loopback: IoStats,
    external: IoStats,
}

impl Add for NetIoStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            loopback: self.loopback + rhs.loopback,
            external: self.external + rhs.external,
        }
    }
}

impl Sub for NetIoStats {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            loopback: self.loopback - rhs.loopback,
            external: self.external - rhs.external,
        }
    }
}

#[derive(Debug)]
struct NetIoSnapshot {
    stats: NetIoStats,
    time: Instant,
}

impl NetIoSnapshot {
    async fn capture() -> Self {
        let nics = heim::net::nic()
            .filter_map(|nic| async {
                nic.ok().filter(|nic| nic.is_up()).map(|nic| {
                    (nic.name().to_owned(), nic.is_loopback())
                })
            })
            .collect::<HashMap<_, _>>()
            .await;
        heim::net::io_counters()
            .filter_map(|counters| async { counters.ok() })
            .fold(NetIoStats::default(), move |mut acc, counters| {
                match nics.get(counters.interface()) {
                    Some(true) => acc.loopback += IoStats::from_net(&counters),
                    Some(false) => acc.external += IoStats::from_net(&counters),
                    None => {}
                }
                async move { acc }
            })
            .map(|stats| Self { stats, time: Instant::now() })
            .await
    }
}

#[derive(Debug)]
struct DiskIoSnapshot {
    stats: IoStats,
    time: Instant,
}

impl DiskIoSnapshot {
    async fn capture() -> Self {
        heim::disk::io_counters_physical()
            .filter_map(|counters| async move {
                Some(IoStats::from_disk(&counters.ok()?))
            })
            .fold(IoStats::default(), |acc, stats| ready(acc + stats))
            .map(|stats| Self { stats, time: Instant::now() })
            .await
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

fn duration(t: heim::units::Time) -> Duration {
    Duration::from_nanos(t.get::<heim::units::time::nanosecond>() as u64)
}

fn bytes(q: heim::units::Information) -> u64 {
    q.get::<heim::units::information::byte>()
}

const CRITICAL_COLOR: Srgb<u8> = palette::named::FIREBRICK;

fn throttle<S>(period: Duration, stream: S) -> impl Stream<Item = S::Item>
where
    S: Stream + Unpin,
{
    let interval = tokio::time::interval(period);
    futures::stream::unfold((interval, stream), |(mut interval, mut stream)| async move {
        interval.tick().await;
        let item = stream.next().await;
        item.map(|item| (item, (interval, stream)))
    })
}
