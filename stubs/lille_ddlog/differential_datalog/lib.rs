//! Stub file for differential_datalog crate.
#![allow(dead_code)]
#![allow(warnings)]
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

// --- `api` module stub ---
pub mod api {
    use super::program::Update;
    use std::collections::HashMap;

    #[derive(Default, Clone, Debug)]
    pub struct DeltaMap(pub HashMap<usize, Vec<Update>>);

    impl IntoIterator for DeltaMap {
        type Item = (usize, Vec<Update>);
        type IntoIter = std::collections::hash_map::IntoIter<usize, Vec<Update>>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

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
            Ok(DeltaMap::default())
        }
    }
}

// --- `record` module stub ---
pub mod record {
    use super::*;

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct DDValue(pub serde_json::Value);

    impl DDValue {
        pub fn into_json(self) -> serde_json::Value {
            self.0
        }
    }

    #[derive(Clone, Debug)]
    pub struct Record;

    #[derive(Clone, Debug)]
    pub enum RelIdentifier {
        RelId(usize),
    }

    pub trait IntoRecord {
        fn into_record(self) -> Record;
    }

    impl IntoRecord for Record {
        fn into_record(self) -> Record {
            self
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
