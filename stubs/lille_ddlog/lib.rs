//! Stub file for lille-ddlog crate.
//! This file is replaced during the build process with generated DDlog code.
//! It exists to satisfy Cargo's dependency resolution during formatting and other operations.

#![allow(dead_code)]

// Minimal stub API mirroring the expected interface of generated DDlog code.

pub mod api {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct DDValue(pub Value);

    impl DDValue {
        pub fn from<T: Serialize>(val: &T) -> Result<Self, serde_json::Error> {
            Ok(Self(serde_json::to_value(val)?))
        }
    }

    #[derive(Clone, Debug)]
    pub struct Update {
        pub relid: usize,
        pub weight: isize,
        pub value: DDValue,
    }

    #[derive(Default, Clone, Debug)]
    pub struct DeltaMap;

    #[derive(Clone, Debug)]
    pub struct HDDlog;

    pub fn run(_workers: usize, _do_store: bool) -> Result<(HDDlog, DeltaMap), String> {
        Ok((HDDlog, DeltaMap))
    }

    impl HDDlog {
        pub fn transaction_start(&self) -> Result<(), String> {
            Ok(())
        }

        pub fn apply_updates<I>(&self, _updates: &mut I) -> Result<(), String>
        where
            I: Iterator<Item = Update>,
        {
            Ok(())
        }

        pub fn transaction_commit_dump_changes(&self) -> Result<DeltaMap, String> {
            Ok(DeltaMap)
        }
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
