// Copyright (C) 2019-2022 Stephane Raux. Distributed under the 0BSD license.

// #![deny(missing_docs)]
// #![deny(warnings)]

pub use block::Block;
pub use event::{events, Event};
pub use serialize::serialize_blocks;
pub use util::{throttle, ticks};

mod block;
mod event;
pub mod pretty;
mod serialize;
pub mod sources;
mod util;
pub mod views;
