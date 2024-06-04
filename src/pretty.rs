// Copyright (C) 2019-2024 Stephane Raux. Distributed under the 0BSD license.

use number_prefix::NumberPrefix;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{self, Display},
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Quantity<'a> {
    value: f64,
    unit: Cow<'a, str>,
}

impl<'a> Quantity<'a> {
    pub fn new<S>(value: f64, unit: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self {
            value,
            unit: unit.into(),
        }
    }
}

impl Display for Quantity<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = &self.unit;
        match NumberPrefix::decimal(self.value) {
            NumberPrefix::Standalone(q) => write!(f, "{q:.1} {unit}"),
            NumberPrefix::Prefixed(prefix, q) => write!(f, "{q:.1} {prefix}{unit}"),
        }
    }
}
