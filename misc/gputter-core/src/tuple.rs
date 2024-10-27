pub trait StaticIndexable<const I: usize> {
    type Output;
    fn get(&self) -> &Self::Output;
}

macro_rules! tuple_idx_impl {
    ($index: tt $selected: ident $($ty:ident),+) => {
        impl<$($ty),*> StaticIndexable<$index> for ($($ty,)*) {
            type Output = $selected;
            fn get(&self) -> &Self::Output {
                &self.$index
            }
        }
    }
}

tuple_idx_impl!(0 A A);

tuple_idx_impl!(0 A A, B);
tuple_idx_impl!(1 B A, B);

tuple_idx_impl!(0 A A, B, C);
tuple_idx_impl!(1 B A, B, C);
tuple_idx_impl!(2 C A, B, C);

tuple_idx_impl!(0 A A, B, C, D);
tuple_idx_impl!(1 B A, B, C, D);
tuple_idx_impl!(2 C A, B, C, D);
tuple_idx_impl!(3 D A, B, C, D);