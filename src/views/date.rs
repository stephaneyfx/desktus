// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

use crate::Block;
use chrono::{DateTime, Local};
use palette::Srgb;

#[derive(Debug)]
pub struct DateView<M> {
    time: DateTime<Local>,
    message: M,
    foreground: Srgb<u8>,
}

impl<M> DateView<M> {
    pub fn new(time: DateTime<Local>, message: M, foreground: Srgb<u8>) -> Self {
        Self {
            time,
            message,
            foreground,
        }
    }
}

impl<M: Clone> DateView<M> {
    pub fn render(&self) -> Block<M> {
        Block::new(
            self.time.format("%a %b %d").to_string(),
            self.foreground,
            self.message.clone(),
        )
    }
}
