// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

use crate::Block;
use palette::Srgb;

#[derive(Debug)]
pub struct CpuView<M> {
    usage: u32,
    message: M,
    foreground: Srgb<u8>,
    critical_usage: u32,
    critical_background: Srgb<u8>,
}

impl<M> CpuView<M> {
    pub fn new(usage: u32, message: M, foreground: Srgb<u8>) -> Self {
        Self {
            usage,
            message,
            foreground,
            critical_usage: 95,
            critical_background: palette::named::FIREBRICK,
        }
    }

    pub fn critical_when_more_than(self, critical_usage: u32) -> Self {
        Self {
            critical_usage,
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

impl<M: Clone> CpuView<M> {
    pub fn render(&self) -> Block<M> {
        let usage = self.usage;
        Block {
            background: (usage > self.critical_usage).then(|| self.critical_background),
            ..Block::new(
                format!("\u{f08d6} {usage}%"),
                self.foreground,
                self.message.clone(),
            )
        }
    }
}
