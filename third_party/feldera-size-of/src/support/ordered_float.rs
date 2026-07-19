#![cfg(feature = "ordered-float")]

use crate::{Context, SizeOf};
use ordered_float::{FloatCore, FloatIsNan, NotNan, OrderedFloat};

impl_total_size_childless! {
    FloatIsNan,
}

impl<T> SizeOf for NotNan<T>
where
    // `FloatCore` (rather than `Float`) is required so that the `NotNan<T>:
    // Deref<Target = T>` coercion below is available: ordered-float 5.x bounds
    // its `Deref` impls on `FloatCore`, not `Float`.
    T: FloatCore + SizeOf,
{
    #[inline]
    fn size_of_children(&self, context: &mut Context) {
        T::size_of_children(self, context);
    }
}

impl<T> SizeOf for OrderedFloat<T>
where
    // See the note on the `NotNan<T>` impl above: ordered-float 5.x bounds its
    // `Deref` impls on `FloatCore`, so the coercion of `&OrderedFloat<T>` to
    // `&T` requires `T: FloatCore`.
    T: FloatCore + SizeOf,
{
    #[inline]
    fn size_of_children(&self, context: &mut Context) {
        T::size_of_children(self, context);
    }
}
