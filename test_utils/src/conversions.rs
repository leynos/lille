//! Conversion helpers for test physics newtypes.
//! Centralises `From` implementations enabling literal usage in tests.

use crate::physics::{
    BlockCoords, BlockId, Coords2D, Coords3D, EntityId, FearValue, ForceVector, Gradient, Mass,
};

macro_rules! impl_newtype_conversions {
    ($name:ident, $ty:ty) => {
        impl From<$ty> for $name {
            fn from(value: $ty) -> Self {
                Self(value)
            }
        }
        impl From<$name> for $ty {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

macro_rules! impl_coords3_conversions {
    ($name:ident, $ty:ty) => {
        impl From<($ty, $ty, $ty)> for $name {
            fn from((x, y, z): ($ty, $ty, $ty)) -> Self {
                Self { x, y, z }
            }
        }
        impl From<$name> for ($ty, $ty, $ty) {
            fn from(coords: $name) -> Self {
                (coords.x, coords.y, coords.z)
            }
        }
    };
}

macro_rules! impl_coords2_conversions {
    ($name:ident, $ty:ty) => {
        impl From<($ty, $ty)> for $name {
            fn from((x, y): ($ty, $ty)) -> Self {
                Self { x, y }
            }
        }
        impl From<$name> for ($ty, $ty) {
            fn from(coords: $name) -> Self {
                (coords.x, coords.y)
            }
        }
    };
}

impl_newtype_conversions!(EntityId, i64);
impl_newtype_conversions!(BlockId, i64);
impl_newtype_conversions!(Mass, f64);
impl_newtype_conversions!(FearValue, f64);
impl_coords3_conversions!(Coords3D, f64);
impl_coords3_conversions!(BlockCoords, i32);
impl_coords2_conversions!(Coords2D, f64);
impl_coords3_conversions!(ForceVector, f64);
impl_coords2_conversions!(Gradient, f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_roundtrip() {
        let id: EntityId = 42_i64.into();
        let raw: i64 = id.into();
        assert_eq!(raw, 42);
    }

    #[test]
    fn block_coords_from_tuple_roundtrip() {
        let coords: BlockCoords = (1, 2, 3).into();
        let tuple: (i32, i32, i32) = coords.into();
        assert_eq!(tuple, (1, 2, 3));
    }

    #[test]
    fn coords3d_from_tuple_roundtrip() {
        let coords: Coords3D = (1.0, 2.0, 3.0).into();
        let tuple: (f64, f64, f64) = coords.into();
        assert_eq!(tuple, (1.0, 2.0, 3.0));
    }

    #[test]
    fn coords2d_from_tuple_roundtrip() {
        let coords: Coords2D = (1.0, 2.0).into();
        let tuple: (f64, f64) = coords.into();
        assert_eq!(tuple, (1.0, 2.0));
    }

    #[test]
    fn fear_value_roundtrip() {
        let fear: FearValue = 0.5_f64.into();
        let raw: f64 = fear.into();
        assert_eq!(raw, 0.5);
    }
}
