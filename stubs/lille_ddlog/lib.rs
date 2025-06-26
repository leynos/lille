//! Stub file for lille-ddlog crate.
//! This file is replaced during the build process with generated DDlog code.
//! It exists to satisfy Cargo's dependency resolution during formatting and other operations.

#![allow(dead_code)]

// Minimal stub API mirroring the expected interface of generated DDlog code.

pub mod api {
    pub use differential_datalog::api::HDDlog;
    pub use differential_datalog::{DDlog, DDlogDynamic};
    pub use differential_datalog::program::{Update, DeltaMap};
    pub use differential_datalog::ddval::DDValue;

    pub fn run(workers: usize, do_store: bool) -> Result<(HDDlog, DeltaMap), String> {
        differential_datalog::api::run(workers, do_store).map_err(|e| e.to_string())
    }
}

pub use api::run;

#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug)]
pub enum Relations {
    Position,
    Velocity,
    Mass,
    Force,
    NewPosition,
    NewVelocity,
}

#[allow(non_upper_case_globals)]
pub mod relations {
    pub const Position: usize = 0;
    pub const Velocity: usize = 1;
    pub const Mass: usize = 2;
    pub const Force: usize = 3;
    pub const NewPosition: usize = 4;
    pub const NewVelocity: usize = 5;
}

use ordered_float::OrderedFloat;
use serde::Serialize;
use differential_datalog::ddval::DDValue;

#[derive(Clone, Debug, Serialize)]
pub enum Record {
    Position {
        entity: i64,
        x: OrderedFloat<f32>,
        y: OrderedFloat<f32>,
        z: OrderedFloat<f32>,
    },
}

pub mod record {
    pub use super::Record;
}

impl From<Record> for DDValue {
    fn from(rec: Record) -> Self {
        DDValue::from(&rec).unwrap_or_default()
    }
}
