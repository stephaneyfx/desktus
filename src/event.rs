// Copyright (C) 2022 Stephane Raux. Distributed under the 0BSD license.

use futures::{Stream, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;
use tokio_stream::wrappers::LinesStream;

#[derive(Clone, Debug, PartialEq)]
pub struct Event<M> {
    pub message: M,
    pub button: Button,
    pub modifiers: Vec<ButtonModifier>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Button {
    MouseLeft,
    MouseMiddle,
    MouseRight,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ButtonModifier {
    Shift,
}

#[derive(Debug, Deserialize)]
struct WireEvent {
    name: String,
    button: u32,
    modifiers: Vec<String>,
}

impl<M: DeserializeOwned> TryFrom<WireEvent> for Event<M> {
    type Error = ();

    fn try_from(e: WireEvent) -> Result<Self, Self::Error> {
        let message = base64::decode(&e.name)
            .map_err(|_| ())
            .and_then(|bytes| bincode::deserialize(&bytes).map_err(|_| ()))?;
        Ok(Self {
            message,
            button: parse_button(e.button).ok_or(())?,
            modifiers: e
                .modifiers
                .into_iter()
                .filter_map(|s| parse_modifier(&s))
                .collect(),
        })
    }
}

fn parse_modifier(s: &str) -> Option<ButtonModifier> {
    match s {
        "Shift" => Some(ButtonModifier::Shift),
        _ => None,
    }
}

fn parse_button(n: u32) -> Option<Button> {
    match n {
        1 => Some(Button::MouseLeft),
        2 => Some(Button::MouseMiddle),
        3 => Some(Button::MouseRight),
        _ => None,
    }
}

pub fn events<M: DeserializeOwned>() -> impl Stream<Item = Event<M>> {
    LinesStream::new(tokio::io::BufReader::new(tokio::io::stdin()).lines()).filter_map(
        |line| async move {
            serde_json::from_str::<WireEvent>(&line.ok()?)
                .ok()?
                .try_into()
                .ok()
        },
    )
}
