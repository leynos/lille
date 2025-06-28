//! Stub file for lille-ddlog crate.
#![allow(dead_code, unused_variables)]
#![allow(warnings)]
#![allow(clippy::all)]

// Stub for the top-level items in the generated crate
pub use differential_datalog::api::{DeltaMap, HDDlog};

pub fn run(
    _workers: usize,
    _do_store: bool,
) -> Result<(HDDlog, DeltaMap<differential_datalog::record::DDValue>), String> {
    Ok((HDDlog, DeltaMap::<differential_datalog::record::DDValue>::default()))
}

// Stub for the Relations enum
#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum Relations {
    entity_state_Position,
    entity_state_Velocity,
    entity_state_Mass,
    entity_state_Target,
    entity_state_Fraidiness,
    entity_state_Meanness,
    physics_Force,
    physics_NewPosition,
    physics_NewVelocity,
}

pub fn relval_from_record(
    _relation: Relations,
    _record: &differential_datalog::record::Record,
) -> Result<differential_datalog::record::DDValue, String> {
    Ok(differential_datalog::record::DDValue::default())
}

pub mod shared {
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

    #[derive(Clone, Debug)]
    pub struct Target {
        pub entity: i64,
        pub tx: OrderedFloat<f32>,
        pub ty: OrderedFloat<f32>,
    }

    #[derive(Clone, Debug)]
    pub struct Fraidiness {
        pub entity: i64,
        pub factor: OrderedFloat<f32>,
    }

    #[derive(Clone, Debug)]
    pub struct Meanness {
        pub entity: i64,
        pub factor: OrderedFloat<f32>,
    }

    impl DDValConvert for Position {
        fn into_ddvalue(self) -> DDValue {
            unimplemented!("stub into_ddvalue")
        }
    }

    impl Position {
        pub fn try_from_ddvalue(_val: DDValue) -> Option<Self> {
            None
        }
    }

    impl IntoRecord for Position {
        fn into_record(self) -> Record {
            unimplemented!("stub into_record")
        }
    }

    impl DDValConvert for Target {
        fn into_ddvalue(self) -> DDValue {
            unimplemented!("stub into_ddvalue")
        }
    }

    impl IntoRecord for Target {
        fn into_record(self) -> Record {
            unimplemented!("stub into_record")
        }
    }

    impl DDValConvert for Fraidiness {
        fn into_ddvalue(self) -> DDValue {
            unimplemented!("stub into_ddvalue")
        }
    }

    impl IntoRecord for Fraidiness {
        fn into_record(self) -> Record {
            unimplemented!("stub into_record")
        }
    }

    impl DDValConvert for Meanness {
        fn into_ddvalue(self) -> DDValue {
            unimplemented!("stub into_ddvalue")
        }
    }

    impl IntoRecord for Meanness {
        fn into_record(self) -> Record {
            unimplemented!("stub into_record")
        }
    }
}

pub mod physics {
    pub use crate::shared::Position as NewPosition;
}

pub mod types__physics {
    pub use crate::shared::Position as NewPosition;
}

pub mod typedefs {
    pub mod entity_state {
        pub use crate::shared::{Fraidiness, Meanness, Position, Target};
    }
    pub mod physics {
        pub use crate::shared::Position as NewPosition;
    }
}

pub mod entity_state {
    pub use crate::shared::{Fraidiness, Meanness, Position, Target};
}

pub mod types__entity_state {
    pub use crate::shared::{Fraidiness, Meanness, Position, Target};
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
        Target {
            entity: i64,
            tx: ordered_float::OrderedFloat<f32>,
            ty: ordered_float::OrderedFloat<f32>,
        },
        Fraidiness {
            entity: i64,
            factor: ordered_float::OrderedFloat<f32>,
        },
        Meanness {
            entity: i64,
            factor: ordered_float::OrderedFloat<f32>,
        },
    }

    impl From<Record> for DDValue {
        fn from(rec: Record) -> Self {
            unimplemented!("stub for From<Record> for DDValue")
        }
    }
}
