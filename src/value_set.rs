extern crate derive_more;
extern crate num;

use std::{mem, ops};

use crate::types::ValueType;

pub trait ValueSet: Copy + Eq {
    const BITS: ValueType = (mem::size_of::<Self>() as ValueType) * (u8::BITS as ValueType);

    fn from_value(value: ValueType) -> Self;

    fn full(num_values: ValueType) -> Self;

    fn empty() -> Self;

    // Return the number of values in the set.
    fn count(&self) -> usize;

    // count() == 0
    fn is_empty(&self) -> bool;

    // count() > 1
    fn has_multiple(&self) -> bool;

    fn min(&self) -> Option<ValueType>;

    // Return the value if it is unique, otherwise None.
    // To get a value more efficiently without checking the count, use min().
    #[inline]
    fn value(&self) -> Option<ValueType> {
        if self.is_empty() || self.has_multiple() {
            return None;
        }
        self.min()
    }

    fn remove_set(&mut self, other: &Self);

    fn add_set(&mut self, other: &Self);

    fn intersection(&self, other: &Self) -> Self;

    fn union(&self, other: &Self) -> Self;

    fn without(&self, other: &Self) -> Self;

    #[inline]
    fn pop(&mut self) -> Option<ValueType> {
        let value = self.min()?;
        self.remove_set(&Self::from_value(value));
        Some(value)
    }
}

pub struct IntBitSet<T>(T);

impl<T> ValueSet for IntBitSet<T>
where
    T: num::PrimInt
        + ops::Shl<ValueType, Output = T>
        + ops::Neg<Output = T>
        + ops::BitAnd<Output = T>
        + ops::BitAndAssign
        + ops::BitOr<Output = T>
        + ops::BitOrAssign
        + num::traits::WrappingSub,
{
    #[inline]
    fn from_value(value: ValueType) -> Self {
        Self(T::one() << value)
    }

    #[inline]
    fn full(num_values: ValueType) -> Self {
        Self(if num_values == (Self::BITS as ValueType) {
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
        self.0 & (self.0.wrapping_sub(&T::one())) != T::zero()
    }

    #[inline]
    fn min(&self) -> Option<ValueType> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.trailing_zeros() as ValueType)
        }
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
}

impl<T: Copy> Copy for IntBitSet<T> {}
impl<T: Copy> Clone for IntBitSet<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: PartialEq> PartialEq for IntBitSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T: Eq> Eq for IntBitSet<T> {}

impl<T> FromIterator<ValueType> for IntBitSet<T>
where
    T: num::PrimInt
        + ops::Shl<ValueType, Output = T>
        + ops::Neg<Output = T>
        + ops::BitAndAssign
        + ops::BitAnd<Output = T>
        + ops::BitOrAssign
        + ops::BitOr<Output = T>
        + num::traits::WrappingSub,
{
    fn from_iter<I: IntoIterator<Item = ValueType>>(iter: I) -> Self {
        iter.into_iter()
            .map(Self::from_value)
            .fold(Self::empty(), |a, b| a.union(&b))
    }
}

pub struct RecValueSet<T>(T, T);

impl<T: ValueSet> ValueSet for RecValueSet<T> {
    #[inline]
    fn from_value(value: ValueType) -> Self {
        if value < (T::BITS as ValueType) {
            Self(T::empty(), T::from_value(value))
        } else {
            Self(T::from_value(value - T::BITS as ValueType), T::empty())
        }
    }

    #[inline]
    fn full(num_values: ValueType) -> Self {
        if num_values < (T::BITS as ValueType) {
            Self(T::empty(), T::full(num_values))
        } else {
            Self(
                T::full(num_values - T::BITS as ValueType),
                T::full(T::BITS as ValueType),
            )
        }
    }

    #[inline]
    fn empty() -> Self {
        Self(T::empty(), T::empty())
    }

    #[inline]
    fn count(&self) -> usize {
        self.0.count() + self.1.count()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }

    #[inline]
    fn has_multiple(&self) -> bool {
        (!self.0.is_empty() && !self.1.is_empty()) || self.0.has_multiple() || self.1.has_multiple()
    }

    #[inline]
    fn min(&self) -> Option<ValueType> {
        self.1
            .min()
            .or_else(|| self.0.min().map(|v| v + T::BITS as ValueType))
    }

    #[inline]
    fn remove_set(&mut self, other: &Self) {
        self.0.remove_set(&other.0);
        self.1.remove_set(&other.1);
    }

    #[inline]
    fn add_set(&mut self, other: &Self) {
        self.0.add_set(&other.0);
        self.1.add_set(&other.1);
    }

    #[inline]
    fn intersection(&self, other: &Self) -> Self {
        Self(self.0.intersection(&other.0), self.1.intersection(&other.1))
    }

    #[inline]
    fn union(&self, other: &Self) -> Self {
        Self(self.0.union(&other.0), self.1.union(&other.1))
    }

    #[inline]
    fn without(&self, other: &Self) -> Self {
        Self(self.0.without(&other.0), self.1.without(&other.1))
    }
}

impl<T: Copy> Copy for RecValueSet<T> {}
impl<T: Copy> Clone for RecValueSet<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: PartialEq> PartialEq for RecValueSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}
impl<T: Eq> Eq for RecValueSet<T> {}

impl<T> FromIterator<ValueType> for RecValueSet<T>
where
    T: ValueSet,
{
    fn from_iter<I: IntoIterator<Item = ValueType>>(iter: I) -> Self {
        iter.into_iter()
            .map(Self::from_value)
            .fold(Self::empty(), |a, b| a.union(&b))
    }
}
