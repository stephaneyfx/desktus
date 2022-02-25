// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

use crate::{sources::BatteryState, Block};
use palette::Srgb;

#[derive(Debug)]
pub struct BatteryView<M> {
    state: BatteryState,
    message: M,
    foreground: Srgb<u8>,
    critical_capacity: u32,
    critical_background: Srgb<u8>,
}

impl<M> BatteryView<M> {
    pub fn new(state: BatteryState, message: M, foreground: Srgb<u8>) -> Self {
        Self {
            state,
            message,
            foreground,
            critical_capacity: 20,
            critical_background: palette::named::FIREBRICK,
        }
    }

    pub fn critical_when_less_than(self, critical_capacity: u32) -> Self {
        Self {
            critical_capacity,
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

impl<M: Clone> BatteryView<M> {
    pub fn render(&self) -> Block<M> {
        let icon = battery_icon(self.state.capacity, self.state.charging);
        let capacity = self.state.capacity;
        Block {
            background: (capacity < self.critical_capacity).then(|| self.critical_background),
            ..Block::new(
                format!("{icon} {capacity}%"),
                self.foreground,
                self.message.clone(),
            )
        }
    }
}

fn battery_icon(capacity: u32, charging: bool) -> char {
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
    let capacity = (capacity as usize * ICONS.len() / 100).min(ICONS.len() - 1);
    ICONS[capacity][charging as usize]
}
