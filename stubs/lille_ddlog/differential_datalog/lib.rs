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

// --- `record` module stub ---
pub mod record {
    use super::*;

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct DDValue(serde_json::Value);

    #[derive(Clone, Debug)]
    pub enum UpdCmd {
        Insert(usize, DDValue),
        Delete(usize, DDValue),
    }

    impl From<super::program::Update> for UpdCmd {
        fn from(_upd: super::program::Update) -> Self {
            unimplemented!("stub for From<Update> for UpdCmd")
        }
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
