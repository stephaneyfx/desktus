// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

use crate::{util::throttle, Block};
use futures::{Stream, StreamExt};
use palette::Srgb;
use serde::{Serialize, Serializer};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct WireBlock {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,
    full_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short_text: Option<String>,
    color: Color,
    #[serde(skip_serializing_if = "Option::is_none")]
    background: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border: Option<Color>,
    urgent: bool,
    separator: bool,
}

impl<M: Serialize> From<Block<M>> for WireBlock {
    fn from(b: Block<M>) -> Self {
        Self {
            name: base64::encode(&bincode::serialize(&b.message).unwrap()),
            instance: None,
            full_text: b.text,
            short_text: None,
            color: b.foreground.into(),
            background: b.background.map(Into::into),
            border: b.border.map(Into::into),
            urgent: false,
            separator: false,
        }
    }
}

#[derive(Debug)]
struct Color(Srgb<u8>);

impl From<Srgb<u8>> for Color {
    fn from(c: Srgb<u8>) -> Self {
        Self(c)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let Srgb {
            red, green, blue, ..
        } = self.0;
        serializer.serialize_str(&format!("#{red:02x}{green:02x}{blue:02x}"))
    }
}

pub async fn serialize_blocks<S, M>(blocks: S)
where
    S: Stream<Item = Vec<Block<M>>>,
    M: Serialize,
{
    println!(r#"{{ "version": 1 }}"#);
    println!("[");
    let blocks = throttle(Duration::from_secs(1), blocks);
    blocks
        .for_each(|blocks| async move {
            let blocks = blocks.into_iter().map(WireBlock::from).collect::<Vec<_>>();
            println!("{}", serde_json::to_string(&blocks).unwrap());
        })
        .await;
    println!("[]");
    println!("]");
}
