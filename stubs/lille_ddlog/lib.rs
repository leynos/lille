//! Stub file for lille-ddlog crate.
//! This file is replaced during the build process with generated DDlog code.
//! It exists to satisfy Cargo's dependency resolution during formatting and other operations.

#![allow(dead_code)]

// Minimal stub API mirroring the expected interface of generated DDlog code.

pub mod api {
    pub use differential_datalog::api::{DDValue, DeltaMap, HDDlog, Update};

    pub fn run(_workers: usize, _do_store: bool) -> Result<(HDDlog, DeltaMap), String> {
        Ok((HDDlog, DeltaMap))
    }
}

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
