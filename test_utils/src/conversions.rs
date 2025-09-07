//! Conversion helpers for test physics newtypes.
//! Centralises `From` implementations enabling literal usage in tests.

use crate::physics::{BlockCoords, BlockId, Coords2D, Coords3D, EntityId, ForceVector, Gradient};

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
impl_coords3_conversions!(Coords3D, f64);
impl_coords3_conversions!(BlockCoords, i32);
impl_coords2_conversions!(Coords2D, f64);
impl_coords3_conversions!(ForceVector, f64);
impl_coords2_conversions!(Gradient, f64);
