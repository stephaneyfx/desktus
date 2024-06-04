// Copyright (C) 2024 Stephane Raux. Distributed under the 0BSD license.

use crate::{sources::BrightnessInfo, Block};
use palette::Srgb;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct BrightnessView<M, F> {
    info: BrightnessInfo,
    make_message: F,
    foreground: Srgb<u8>,
    message: PhantomData<fn() -> M>,
}

impl<M, F> BrightnessView<M, F>
where
    F: Fn(&str) -> M,
{
    pub fn new(info: BrightnessInfo, make_message: F, foreground: Srgb<u8>) -> Self {
        Self {
            info,
            make_message,
            foreground,
            message: PhantomData,
        }
    }

    pub fn render(&self) -> Block<M> {
        let level = self.info.level;
        Block::new(
            format!("\u{f00e0} {level}%"),
            self.foreground,
            (self.make_message)(&self.info.device),
        )
    }
}
