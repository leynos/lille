pub mod record {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct DDValue(pub Value);

    impl DDValue {
        pub fn from<T: Serialize>(val: &T) -> Result<Self, serde_json::Error> {
            Ok(Self(serde_json::to_value(val)?))
        }
    }
}

pub mod program {
    use super::record::DDValue;

    #[derive(Clone, Debug)]
    pub struct Update {
        pub relid: usize,
        pub weight: isize,
        pub value: DDValue,
    }

    #[derive(Default, Clone, Debug)]
    pub struct DeltaMap;

    pub trait DDlog {}
    pub trait DDlogDynamic {}
}

pub mod ddval {
    pub use super::record::DDValue;
}

pub use record::DDValue;
pub use program::{DeltaMap, Update, DDlog, DDlogDynamic};

pub use api::run;

pub mod api {
    use super::program::{DeltaMap, Update};

    #[derive(Clone, Debug)]
    pub struct HDDlog;

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

        pub fn apply_updates_dynamic<I>(&self, updates: &mut I) -> Result<(), String>
        where
            I: Iterator<Item = Update>,
        {
            self.apply_updates(updates)
        }

        pub fn transaction_commit_dump_changes(&self) -> Result<DeltaMap, String> {
            Ok(DeltaMap)
        }

        pub fn transaction_commit_dump_changes_dynamic(&self) -> Result<DeltaMap, String> {
            self.transaction_commit_dump_changes()
        }
    }

    pub fn run(_workers: usize, _do_store: bool) -> Result<(HDDlog, DeltaMap), String> {
        Err("unimplemented".to_string())
    }
}
