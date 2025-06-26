//! Stub file for differential_datalog crate.
#![allow(dead_code)]
#![allow(warnings)]
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

// --- `api` module stub ---
pub mod api {
    #[derive(Default, Clone, Debug)]
    pub struct DeltaMap;

    #[derive(Clone, Debug)]
    pub struct HDDlog;

    impl HDDlog {
        pub fn transaction_start(&self) -> Result<(), String> {
            Ok(())
        }

        pub fn apply_updates_dynamic<I>(
            &mut self,
            _updates: I,
        ) -> Result<(), String>
        where
            I: IntoIterator<Item = super::record::UpdCmd>,
        {
            Ok(())
        }

        pub fn transaction_commit_dump_changes_dynamic(&mut self) -> Result<DeltaMap, String> {
            Ok(DeltaMap)
        }
    }
}

impl api::DeltaMap {
    pub fn clear_rel(&mut self, _relid: usize) -> Vec<(record::DDValue, isize)> {
        Vec::new()
    }
}

// --- `record` module stub ---
pub mod record {
    use super::*;

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct DDValue(serde_json::Value);

    #[derive(Clone, Debug)]
    pub struct Record;

    #[derive(Clone, Debug)]
    pub enum RelIdentifier {
        RelId(usize),
    }

    pub trait IntoRecord {
        fn into_record(self) -> Record;
    }

    pub trait FromRecord: Sized {
        fn from_record(_val: &Record) -> Result<Self, String>;
    }

    impl IntoRecord for Record {
        fn into_record(self) -> Record {
            self
        }
    }

    impl FromRecord for Record {
        fn from_record(val: &Record) -> Result<Self, String> {
            Ok(val.clone())
        }
    }

    impl IntoRecord for DDValue {
        fn into_record(self) -> Record {
            Record
        }
    }

    #[derive(Clone, Debug)]
    pub enum UpdCmd {
        Insert(RelIdentifier, Record),
        Delete(RelIdentifier, Record),
    }
}

// --- `ddval` module stub ---
pub mod ddval {
    use super::record::DDValue;

    pub trait DDValConvert {
        fn into_ddvalue(self) -> DDValue;
    }
}

pub use ddval::DDValConvert;
pub use record::FromRecord;

// --- `program` module stub ---
pub mod program {
    use super::record::DDValue;

    #[derive(Clone, Debug)]
    pub enum Update {
        Insert { relid: usize, v: DDValue },
        Delete { relid: usize, v: DDValue },
    }

    pub trait DDlog {
        // Stub trait
    }
    pub trait DDlogDynamic: DDlog {
        // Stub trait
    }
}

pub use program::{DDlog, DDlogDynamic};
