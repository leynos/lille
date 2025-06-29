//! Stub file for differential_datalog crate.
#![allow(dead_code)]
#![allow(warnings)]
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

// --- `api` module stub ---
pub mod api {
    use std::collections::{BTreeMap, BTreeSet};
    use std::marker::PhantomData;
    use super::record::{DDValue, Record, UpdCmd};
    use super::program::{DDlog, DDlogDynamic, Update};

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

    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone, Debug)]
    pub struct HDDlog;

    /// Counts the number of times [`HDDlog::stop`] has been called.
    pub static STOP_CALLS: AtomicUsize = AtomicUsize::new(0);

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
            STOP_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    impl super::program::DDlog for HDDlog {
        fn transaction_commit_dump_changes(
            &self,
        ) -> Result<DeltaMap<DDValue>, String> {
            Ok(DeltaMap::default())
        }

        fn apply_updates(
            &self,
            _upds: &mut dyn Iterator<Item = super::program::Update>,
        ) -> Result<(), String> {
            Ok(())
        }

        fn query_index(
            &self,
            _index: usize,
            _key: DDValue,
        ) -> Result<BTreeSet<DDValue>, String> {
            Ok(BTreeSet::new())
        }

        fn dump_index(
            &self,
            _index: usize,
        ) -> Result<BTreeSet<DDValue>, String> {
            Ok(BTreeSet::new())
        }
    }

    impl super::program::DDlogDynamic for HDDlog {
        fn transaction_start(&mut self) -> Result<(), String> {
            self.transaction_start()
        }

        fn transaction_commit_dump_changes_dynamic(
            &mut self,
        ) -> Result<std::collections::BTreeMap<usize, Vec<(Record, isize)>>, String> {
            Ok(std::collections::BTreeMap::new())
        }

        fn transaction_commit(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn transaction_rollback(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn apply_updates_dynamic(
            &mut self,
            _upds: &mut dyn Iterator<Item = UpdCmd>,
        ) -> Result<(), String> {
            Ok(())
        }

        fn clear_relation(&mut self, _table: usize) -> Result<(), String> {
            Ok(())
        }

        fn query_index_dynamic(
            &mut self,
            _index: usize,
            _key: &Record,
        ) -> Result<Vec<Record>, String> {
            Ok(Vec::new())
        }

        fn dump_index_dynamic(&mut self, _index: usize) -> Result<Vec<Record>, String> {
            Ok(Vec::new())
        }

        fn stop(&mut self) -> Result<(), String> {
            STOP_CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
}

pub use api::{DeltaMap, STOP_CALLS};

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
    use super::api::DeltaMap;
    use super::record::{DDValue, Record, UpdCmd};
    use std::collections::BTreeSet;

    #[derive(Clone, Debug)]
    pub enum Update {
        Insert { relid: usize, v: DDValue },
        Delete { relid: usize, v: DDValue },
    }

    pub trait DDlog {
        fn transaction_commit_dump_changes(
            &self,
        ) -> Result<DeltaMap<DDValue>, String>;
        fn apply_updates(
            &self,
            upds: &mut dyn Iterator<Item = Update>,
        ) -> Result<(), String>;
        fn query_index(
            &self,
            index: usize,
            key: DDValue,
        ) -> Result<BTreeSet<DDValue>, String>;
        fn dump_index(&self, index: usize) -> Result<BTreeSet<DDValue>, String>;
    }
    pub trait DDlogDynamic: DDlog {
        fn transaction_start(&mut self) -> Result<(), String>;
        fn transaction_commit_dump_changes_dynamic(
            &mut self,
        ) -> Result<std::collections::BTreeMap<usize, Vec<(Record, isize)>>, String>;
        fn transaction_commit(&mut self) -> Result<(), String>;
        fn transaction_rollback(&mut self) -> Result<(), String>;
        fn apply_updates_dynamic(
            &mut self,
            upds: &mut dyn Iterator<Item = UpdCmd>,
        ) -> Result<(), String>;
        fn clear_relation(&mut self, table: usize) -> Result<(), String>;
        fn query_index_dynamic(&mut self, index: usize, key: &Record) -> Result<Vec<Record>, String>;
        fn dump_index_dynamic(&mut self, index: usize) -> Result<Vec<Record>, String>;
        fn stop(&mut self) -> Result<(), String>;
    }
}

pub use program::{DDlog, DDlogDynamic};
