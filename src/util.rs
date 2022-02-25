// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

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
