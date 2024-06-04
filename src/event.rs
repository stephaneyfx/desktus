// Copyright (C) 2024 Stephane Raux. Distributed under the 0BSD license.

use futures::{Stream, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;
use tokio_stream::wrappers::LinesStream;

#[derive(Clone, Debug, PartialEq)]
pub struct Event<M> {
    pub message: M,
    pub button: MouseButton,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Down,
    Up,
}

#[derive(Debug, Deserialize)]
struct WireEvent {
    name: String,
    button: u32,
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
        })
    }
}

fn parse_button(n: u32) -> Option<MouseButton> {
    match n {
        1 => Some(MouseButton::Left),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Right),
        4 => Some(MouseButton::Down),
        5 => Some(MouseButton::Up),
        _ => None,
    }
}

pub fn events<M: DeserializeOwned>() -> impl Stream<Item = Event<M>> {
    LinesStream::new(tokio::io::BufReader::new(tokio::io::stdin()).lines()).filter_map(
        |line| async move {
            let line = line.ok()?;
            let line = line.trim_matches(&[','][..]);
            serde_json::from_str::<WireEvent>(&line)
                .ok()?
                .try_into()
                .ok()
        },
    )
}
