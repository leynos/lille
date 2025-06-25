pub mod api {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    pub mod ddval {
        use super::*;

        #[derive(Clone, Debug, Default, Serialize, Deserialize)]
        pub struct DDValue(pub Value);

        impl DDValue {
            pub fn from<T: Serialize>(val: &T) -> Result<Self, serde_json::Error> {
                Ok(Self(serde_json::to_value(val)?))
            }
        }
    }

    pub mod program {
        use super::ddval::DDValue;

        #[derive(Clone, Debug)]
        pub struct Update {
            pub relid: usize,
            pub weight: isize,
            pub value: DDValue,
        }
    }

    pub use ddval::DDValue;
    pub use program::Update;

    #[derive(Default, Clone, Debug)]
    pub struct DeltaMap;

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

        pub fn transaction_commit_dump_changes(&self) -> Result<DeltaMap, String> {
            Ok(DeltaMap)
        }
    }

    pub fn run(_workers: usize, _do_store: bool) -> Result<(HDDlog, DeltaMap), String> {
        Err("unimplemented".to_string())
    }
}
