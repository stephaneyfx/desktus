// Copyright (C) 2019-2024 Stephane Raux. Distributed under the 0BSD license.

pub use battery::BatteryView;
pub use brightness::BrightnessView;
pub use cpu::CpuView;
pub use date::DateView;
pub use disk::DiskView;
pub use memory::MemoryView;
pub use time::TimeView;

mod battery;
mod brightness;
mod cpu;
mod date;
mod disk;
mod memory;
mod time;
