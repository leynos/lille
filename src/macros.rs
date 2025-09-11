//! Macros for reducing boilerplate in DBSP record types.
//!
//! Provides `dbsp_record!`, which defines a struct with standard trait
//! derivations for archivable, ordered data records used in DBSP streams.
//!
//! # Examples
//!
//! ```rust
//! use lille::dbsp_record;
//!
//! dbsp_record! {
//!     /// Example record
//!     pub struct Example {
//!         pub value: i32,
//!     }
//! }
//!
//! // Derive additional traits (any derive path; trailing comma allowed)
//! dbsp_record! {
//!     /// Example needing Copy
//!     pub struct ExampleCopy {
//!         pub value: i32,
//!     }, serde::Serialize, Copy,
//! }
//! ```
//!
//! The macro expands to a struct deriving serialisation, ordering, and size
//! accounting traits required by the circuit. Optional traits can be
//! appended after the struct definition as derive paths (e.g.,
//! `serde::Serialize`), separated by commas, with an optional trailing comma.
//!
//! # Note
//! If the `Copy` trait is omitted from the list of optional traits, the
//! generated struct will not implement `Copy`. This means instances of the
//! struct cannot be implicitly copied and must be explicitly cloned where
//! required. Users upgrading from earlier versions should ensure their
//! code does not rely on implicit copying or include `Copy` in the trait
//! list if necessary.
//!
//! Additionally, deriving `Copy` places a `Copy` bound on any generic
//! parameters of the generated struct, and is only permitted when all
//! fields implement `Copy` and the type does not implement `Drop`.
#[macro_export]
macro_rules! dbsp_record {
    ($(#[$meta:meta])* $vis:vis struct $name:ident { $($fields:tt)* } $(, $extra:path)* $(,)? ) => {
        $(#[$meta])*
        #[derive(
            ::rkyv::Archive,
            ::rkyv::Serialize,
            ::rkyv::Deserialize,
            Clone,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Default,
            ::size_of::SizeOf
            $(, $extra)*
        )]
        #[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
        $vis struct $name { $($fields)* }
    };
}
