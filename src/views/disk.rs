// Copyright (C) 2024 Stephane Raux. Distributed under the 0BSD license.

use crate::{pretty::Quantity, sources::DiskUsage, util::pie_chart, Block};
use palette::Srgb;

#[derive(Debug)]
pub struct DiskView<M> {
    usage: DiskUsage,
    message: M,
    foreground: Srgb<u8>,
    critical_availability: u64,
    critical_background: Srgb<u8>,
}

impl<M> DiskView<M> {
    pub fn new(usage: DiskUsage, message: M, foreground: Srgb<u8>) -> Self {
        Self {
            usage,
            message,
            foreground,
            critical_availability: 1_000_000_000,
            critical_background: palette::named::FIREBRICK,
        }
    }

    pub fn critical_when_less_than(self, critical_availability: u64) -> Self {
        Self {
            critical_availability,
            ..self
        }
    }

    pub fn critical_background(self, background: Srgb<u8>) -> Self {
        Self {
            critical_background: background,
            ..self
        }
    }
}

impl<M: Clone> DiskView<M> {
    pub fn render(&self) -> Block<M> {
        let available = self.usage.total - self.usage.used;
        let critical = available < self.critical_availability;
        let used = Quantity::new(self.usage.used as f64, "B");
        let pie = pie_chart((self.usage.used * 100 / self.usage.total) as u32);
        Block {
            background: critical.then(|| self.critical_background),
            ..Block::new(
                format!("\u{e5fe} {used} {pie}"),
                self.foreground,
                self.message.clone(),
            )
        }
    }
}
