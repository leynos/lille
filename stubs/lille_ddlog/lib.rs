//! Stub file for lille-ddlog crate.
#![allow(dead_code, unused_variables)]
#![allow(warnings)]
#![allow(clippy::all)]

// Stub for the top-level items in the generated crate
pub use differential_datalog::api::{DeltaMap, HDDlog};

pub fn run(_workers: usize, _do_store: bool) -> Result<(HDDlog, DeltaMap), String> {
    Ok((HDDlog, DeltaMap))
}

// Stub for the Relations enum
#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum Relations {
    entity_state_Position,
    entity_state_Velocity,
    entity_state_Mass,
    physics_Force,
    physics_NewPosition,
    physics_NewVelocity,
}

pub mod entity_state {
    use ordered_float::OrderedFloat;
    use differential_datalog::ddval::DDValConvert;
    use differential_datalog::record::{DDValue, IntoRecord, Record};

    #[derive(Clone, Debug)]
    pub struct Position {
        pub entity: i64,
        pub x: OrderedFloat<f32>,
        pub y: OrderedFloat<f32>,
        pub z: OrderedFloat<f32>,
    }

    impl DDValConvert for Position {
        fn into_ddvalue(self) -> DDValue {
            unimplemented!("stub into_ddvalue")
        }
    }

    impl IntoRecord for Position {
        fn into_record(self) -> Record {
            unimplemented!("stub into_record")
        }
    }
}

pub mod types__entity_state {
    pub use super::entity_state::Position;
}

pub mod typedefs {
    pub mod entity_state {
        pub use crate::entity_state::Position;
    }
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
