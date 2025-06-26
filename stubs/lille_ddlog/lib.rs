//! Stub file for lille-ddlog crate.
#![allow(dead_code, unused_variables)]

// Stub for the top-level items in the generated crate
pub use differential_datalog::api::{DeltaMap, HDDlog};

pub fn run(workers: usize, do_store: bool) -> Result<(HDDlog, DeltaMap), String> {
    Err("not implemented in stub".to_string())
}

// Stub for the Relations enum
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

// Stub for the record module and Record enum
pub mod record {
    use differential_datalog::record::DDValue;
    use serde::Serialize;

    #[derive(Clone, Debug, Serialize)]
    pub enum Record {
        Position {
            entity: i64,
            x: ordered_float::OrderedFloat<f32>,
            y: ordered_float::OrderedFloat<f32>,
            z: ordered_float::OrderedFloat<f32>,
        },
    }

    impl From<Record> for DDValue {
        fn from(rec: Record) -> Self {
            unimplemented!("stub for From<Record> for DDValue")
        }
    }
}
