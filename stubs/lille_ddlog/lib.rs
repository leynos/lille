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
