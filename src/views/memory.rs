// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

use crate::{pretty::Quantity, sources::MemoryUsage, Block};
use palette::Srgb;

#[derive(Debug)]
pub struct MemoryView<M> {
    usage: MemoryUsage,
    message: M,
    foreground: Srgb<u8>,
    critical_availability: u64,
    critical_background: Srgb<u8>,
}

impl<M> MemoryView<M> {
    pub fn new(usage: MemoryUsage, message: M, foreground: Srgb<u8>) -> Self {
        Self {
            usage,
            message,
            foreground,
            critical_availability: 500_000_000,
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

impl<M: Clone> MemoryView<M> {
    pub fn render(&self) -> Block<M> {
        let available = self.usage.total - self.usage.used;
        let critical = available < self.critical_availability;
        let available = Quantity::new(available as f64, "B");
        Block {
            background: critical.then(|| self.critical_background),
            ..Block::new(
                format!("\u{f035b} {available}"),
                self.foreground,
                self.message.clone(),
            )
        }
    }
}
