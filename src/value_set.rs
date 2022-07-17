extern crate derive_more;
extern crate num;

use std::{mem, ops};

pub trait ValueSet {
    fn from_value(value: u8) -> Self;

    fn full(num_values: u8) -> Self;

    fn empty() -> Self;

    // Return the number of values in the set.
    fn count(&self) -> usize;

    // count() == 0
    fn is_empty(&self) -> bool;

    // count() > 1
    fn has_multiple(&self) -> bool;

    fn min(&self) -> Option<u8>;

    // Return the value if it is unique, otherwise None.
    // To get a value more efficiently without checking the count, use min().
    fn value(&self) -> Option<u8>;

    fn remove_set(&mut self, other: &Self);

    fn add_set(&mut self, other: &Self);

    fn intersection(&self, other: &Self) -> Self;

    fn union(&self, other: &Self) -> Self;

    fn without(&self, other: &Self) -> Self;

    fn equals(&self, other: &Self) -> bool;

    fn pop(&mut self) -> Option<u8>;
}

pub struct IntBitSet<T>(T);

impl<T> IntBitSet<T> {
    pub const BITS: u8 = (mem::size_of::<Self>() as u8) * (u8::BITS as u8);
}

impl<T> ValueSet for IntBitSet<T>
where
    T: num::PrimInt
        + ops::Shl<u8, Output = T>
        + ops::Neg<Output = T>
        + ops::BitAnd<Output = T>
        + ops::BitAndAssign
        + ops::BitOr<Output = T>
        + ops::BitOrAssign,
{
    #[inline]
    fn from_value(value: u8) -> Self {
        Self(T::one() << value)
    }

    #[inline]
    fn full(num_values: u8) -> Self {
        Self(if num_values == Self::BITS {
            -T::one()
        } else {
            !(-T::one() << num_values)
        })
    }

    #[inline]
    fn empty() -> Self {
        Self(T::zero())
    }

    #[inline]
    fn count(&self) -> usize {
        self.0.count_ones() as usize
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0 == T::zero()
    }

    #[inline]
    fn has_multiple(&self) -> bool {
        self.0 & (self.0 - T::one()) != T::zero()
    }

    #[inline]
    fn min(&self) -> Option<u8> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.trailing_zeros() as u8)
        }
    }

    #[inline]
    fn value(&self) -> Option<u8> {
        if self.is_empty() || self.has_multiple() {
            return None;
        }
        self.min()
    }

    #[inline]
    fn remove_set(&mut self, other: &Self) {
        self.0 &= !other.0
    }

    #[inline]
    fn add_set(&mut self, other: &Self) {
        self.0 |= other.0
    }

    #[inline]
    fn intersection(&self, other: &Self) -> Self {
        Self(self.0 & other.0)
    }

    #[inline]
    fn union(&self, other: &Self) -> Self {
        Self(self.0 | other.0)
    }

    #[inline]
    fn without(&self, other: &Self) -> Self {
        Self(self.0 & !other.0)
    }

    fn equals(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[inline]
    fn pop(&mut self) -> Option<u8> {
        let value = self.min()?;
        self.remove_set(&Self::from_value(value));
        Some(value)
    }
}

impl<T: Copy> Copy for IntBitSet<T> {}
impl<T: Copy> Clone for IntBitSet<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> FromIterator<u8> for IntBitSet<T>
where
    T: num::PrimInt
        + ops::Shl<u8, Output = T>
        + ops::Neg<Output = T>
        + ops::BitAndAssign
        + ops::BitAnd<Output = T>
        + ops::BitOrAssign
        + ops::BitOr<Output = T>,
{
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        iter.into_iter()
            .map(Self::from_value)
            .fold(Self::empty(), |a, b| a.union(&b))
    }
}
