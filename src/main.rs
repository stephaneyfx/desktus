// Copyright (C) 2019 Stephane Raux. Distributed under the MIT license.

#![deny(warnings)]

use chrono::Local;
use rgb::RGB8;
use serde::{Serialize, Serializer};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

fn main() {
    let get_proceed = Arc::new(AtomicBool::new(true));
    let set_proceed = get_proceed.clone();
    ctrlc::set_handler(move || set_proceed.store(false, Ordering::SeqCst)).unwrap();
    let sources = [
        render_time,
    ];
    println!(r#"{{ "version": 1 }}"#);
    println!("[");
    while get_proceed.load(Ordering::SeqCst) {
        let line = sources.iter().map(|f| f()).collect::<Vec<_>>();
        let line = serde_json::to_string(&line).unwrap();
        println!("{},", line);
        thread::sleep(Duration::from_secs(1));
    }
    println!("[]");
    println!("]");
}

#[derive(Debug, Serialize)]
struct Block {
    full_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_color")]
    color: Option<RGB8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_color")]
    background: Option<RGB8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_color")]
    border: Option<RGB8>,
    urgent: bool,
    separator: bool,
}

impl Block {
    fn new<S: Into<String>>(s: S) -> Block {
        Block {
            full_text: s.into(),
            short_text: None,
            color: None,
            background: None,
            border: None,
            urgent: false,
            separator: true,
        }
    }
}

fn serialize_color<S>(color: &Option<RGB8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = color.as_ref().map(|RGB8 { r, g, b }| format!("#{:02x}{:02x}{:02x}", r, g, b))
        .unwrap_or(String::new());
    serializer.serialize_str(&s)
}

fn render_time() -> Block {
    let t = Local::now();
    Block::new(t.format("%F %R").to_string())
}
