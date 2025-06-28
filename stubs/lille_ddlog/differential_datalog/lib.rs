//! Stub file for differential_datalog crate.
#![allow(dead_code)]
#![allow(warnings)]
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

// --- `api` module stub ---
pub mod api {
    use std::collections::BTreeMap;
    use std::marker::PhantomData;

    #[derive(Clone, Debug)]
    pub struct DeltaMap<V>(PhantomData<V>);

    impl<V> Default for DeltaMap<V> {
        fn default() -> Self {
            DeltaMap(PhantomData)
        }
    }

    impl<V> DeltaMap<V> {
        pub fn try_get_rel(
            &self,
            _relid: usize,
        ) -> Option<&BTreeMap<super::record::DDValue, isize>> {
            None
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

        pub fn transaction_commit_dump_changes_dynamic(
            &mut self,
        ) -> Result<std::collections::BTreeMap<usize, Vec<(super::record::Record, isize)>>, String> {
            Ok(std::collections::BTreeMap::new())
        }

        pub fn stop(self) -> Result<(), String> {
            Ok(())
        }
    }
}

pub use api::DeltaMap;

// Provide a `valmap` module for compatibility with the real crate.
pub mod valmap {
    pub use crate::api::DeltaMap;
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

    impl From<Record> for DDValue {
        fn from(_rec: Record) -> Self {
            DDValue(serde_json::Value::Null)
        }
    }
}

// --- `ddval` module stub ---
pub mod ddval {
    pub use super::record::DDValue;

    pub trait DDValConvert {
        fn into_ddvalue(self) -> DDValue;
        fn try_from_ddvalue(_val: DDValue) -> Option<Self>
        where
            Self: Sized,
        {
            None
        }
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
