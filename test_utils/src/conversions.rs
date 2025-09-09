//! Conversion helpers for test physics newtypes.
//! Centralises `From` implementations enabling literal usage in tests.
//!
//! # Examples
//! ```
//! # use test_utils::conversions::*;
//! # use test_utils::physics::{EntityId, Coords3D, Coords2D, FearValue};
//! let id: EntityId = 1_i64.into();
//! let p: Coords3D = (0.0, 0.0, 1.0).into();
//! let t: Coords2D = (1.0, 1.0).into();
//! let fear: FearValue = 0.5.into();
//! let (x, y): (f64, f64) = t.into();
//! assert_eq!(x, 1.0);
//! ```

use crate::physics::{
    BlockCoords, BlockId, Coords2D, Coords3D, EntityId, FearValue, ForceVector, Gradient, Mass,
};

macro_rules! impl_newtype_conversions {
    ($name:ident, $ty:ty $(, $extra:ty)*) => {
        impl From<$ty> for $name {
            fn from(value: $ty) -> Self {
                Self(value)
            }
        }
        $(impl From<$extra> for $name {
            fn from(value: $extra) -> Self {
                Self(<$ty>::from(value))
            }
        })*
        impl From<$name> for $ty {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

macro_rules! impl_coords3_conversions {
    ($name:ident, $ty:ty) => {
        impl<TX, TY, TZ> From<(TX, TY, TZ)> for $name
        where
            TX: Into<$ty>,
            TY: Into<$ty>,
            TZ: Into<$ty>,
        {
            fn from((x, y, z): (TX, TY, TZ)) -> Self {
                Self {
                    x: x.into(),
                    y: y.into(),
                    z: z.into(),
                }
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
        impl<TX, TY> From<(TX, TY)> for $name
        where
            TX: Into<$ty>,
            TY: Into<$ty>,
        {
            fn from((x, y): (TX, TY)) -> Self {
                Self {
                    x: x.into(),
                    y: y.into(),
                }
            }
        }
        impl From<$name> for ($ty, $ty) {
            fn from(coords: $name) -> Self {
                (coords.x, coords.y)
            }
        }
    };
}

impl_newtype_conversions!(EntityId, i64, i32);
impl_newtype_conversions!(BlockId, i64, i32);
impl_newtype_conversions!(Mass, f64, f32);
impl_newtype_conversions!(FearValue, f64, f32);
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
        let id: EntityId = 42.into();
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
        let coords: Coords3D = (1, 2, 3).into();
        let tuple: (f64, f64, f64) = coords.into();
        assert_eq!(tuple, (1.0, 2.0, 3.0));
    }

    #[test]
    fn coords2d_from_tuple_roundtrip() {
        let coords: Coords2D = (1, 2).into();
        let tuple: (f64, f64) = coords.into();
        assert_eq!(tuple, (1.0, 2.0));
    }

    #[test]
    fn mass_roundtrip() {
        let mass: Mass = 1.0_f32.into();
        let raw: f64 = mass.into();
        assert_eq!(raw, 1.0);
    }

    #[test]
    fn fear_value_roundtrip() {
        let fear: FearValue = 0.5_f32.into();
        let raw: f64 = fear.into();
        assert_eq!(raw, 0.5);
    }
}
