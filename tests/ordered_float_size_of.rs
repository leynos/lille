//! Compile-time regression guard for the ordered-float 5.x / `feldera-size-of`
//! compatibility fix.
//!
//! DBSP records that embed `OrderedFloat<f64>` (such as [`Position`] and
//! [`BlockSlope`]) derive [`SizeOf`], which DBSP requires for its Z-set data.
//! `feldera-size-of` 0.1.x pins its optional `ordered-float` dependency at
//! `^3.0.0`, so its `SizeOf` impl did not apply to the ordered-float 5.x used by
//! this crate; deriving `SizeOf` therefore failed with `E0277`. A vendored fork
//! of `feldera-size-of` widens that constraint (see `third_party/`), and these
//! assertions lock the behaviour in so a regression fails to compile.

use lille::components::BlockSlope;
use lille::dbsp_circuit::Position;
use ordered_float::OrderedFloat;
use size_of::SizeOf;
use static_assertions::assert_impl_all;

// The root cause: the wrapper itself must implement `SizeOf`.
assert_impl_all!(OrderedFloat<f64>: SizeOf);

// The DBSP records that embed it, which previously failed the derive.
assert_impl_all!(Position: SizeOf);
assert_impl_all!(BlockSlope: SizeOf);
