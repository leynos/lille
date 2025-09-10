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
//! // Derive additional traits
//! dbsp_record! {
//!     /// Example needing Copy
//!     pub struct ExampleCopy {
//!         pub value: i32,
//!     }, Copy
//! }
//! ```
//!
//! The macro expands to a struct deriving serialisation, ordering, and size
//! accounting traits required by the circuit. Optional traits can be
//! appended after the struct definition.
#[macro_export]
macro_rules! dbsp_record {
    ($(#[$meta:meta])* $vis:vis struct $name:ident { $($fields:tt)* } $(, $extra:ident)* ) => {
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
