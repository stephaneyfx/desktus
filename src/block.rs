// Copyright (C) 2019-2024 Stephane Raux. Distributed under the 0BSD license.

use palette::Srgb;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block<M> {
    pub message: M,
    pub text: String,
    pub foreground: Srgb<u8>,
    pub background: Option<Srgb<u8>>,
    pub border: Option<Srgb<u8>>,
}

impl<M> Block<M> {
    pub fn new<S>(text: S, foreground: Srgb<u8>, message: M) -> Self
    where
        S: Into<String>,
    {
        Self {
            message,
            text: text.into(),
            foreground,
            background: None,
            border: None,
        }
    }
}
