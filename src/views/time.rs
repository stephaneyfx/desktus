// Copyright (C) 2019-2024 Stephane Raux. Distributed under the 0BSD license.

use crate::Block;
use chrono::{DateTime, Local};
use palette::Srgb;

#[derive(Debug)]
pub struct TimeView<M> {
    time: DateTime<Local>,
    message: M,
    foreground: Srgb<u8>,
}

impl<M> TimeView<M> {
    pub fn new(time: DateTime<Local>, message: M, foreground: Srgb<u8>) -> Self {
        Self {
            time,
            message,
            foreground,
        }
    }
}

impl<M: Clone> TimeView<M> {
    pub fn render(&self) -> Block<M> {
        Block::new(
            self.time.format("%R").to_string(),
            self.foreground,
            self.message.clone(),
        )
    }
}
