// Copyright (C) 2019-2024 Stephane Raux. Distributed under the 0BSD license.

use futures::{Stream, StreamExt};
use std::time::{Duration, Instant};
use tokio::time::MissedTickBehavior;
use tokio_stream::wrappers::IntervalStream;

pub fn ticks(d: Duration) -> impl Stream<Item = Instant> {
    let mut interval = tokio::time::interval(d);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    IntervalStream::new(interval).map(Into::into)
}

pub fn throttle<S: Stream>(d: Duration, stream: S) -> impl Stream<Item = S::Item> {
    stream.zip(ticks(d)).map(|(x, _)| x)
}

pub fn pie_chart(percentage: u32) -> char {
    const SYMBOLS: &[char] = &[
        '\u{f0130}',
        '\u{f0a9e}',
        '\u{f0a9f}',
        '\u{f0aa0}',
        '\u{f0aa1}',
        '\u{f0aa2}',
        '\u{f0aa3}',
        '\u{f0aa4}',
        '\u{f0aa5}',
    ];
    SYMBOLS[(percentage as usize * SYMBOLS.len() / 100).min(SYMBOLS.len() - 1)]
}
