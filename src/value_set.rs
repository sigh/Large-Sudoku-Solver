extern crate derive_more;
extern crate num;

use std::{fmt, mem, ops};

pub trait ValueSet {
    fn from_value(value: u8) -> Self;

    fn full(num_values: u8) -> Self;

    fn empty() -> Self;

    fn is_empty(&self) -> bool;

    fn count(&self) -> usize;

    fn min(&self) -> Option<u8>;

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
    fn is_empty(&self) -> bool {
        self.0 == T::zero()
    }

    #[inline]
    fn count(&self) -> usize {
        self.0.count_ones() as usize
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
        if self.count() != 1 {
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

impl<T: fmt::Debug> fmt::Debug for IntBitSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntBitSet").field(&self.0).finish()
    }
}

impl<T: fmt::Display> fmt::Display for IntBitSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: ops::Not<Output = T>> ops::Not for IntBitSet<T> {
    type Output = IntBitSet<T>;

    fn not(self) -> Self::Output {
        IntBitSet::<T>(!self.0)
    }
}

impl<T: ops::BitOr<Output = T>> ops::BitOr for IntBitSet<T> {
    type Output = IntBitSet<T>;

    fn bitor(self, rhs: Self) -> Self::Output {
        IntBitSet::<T>(self.0 | rhs.0)
    }
}

impl<T: ops::BitOrAssign> ops::BitOrAssign for IntBitSet<T> {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl<T: ops::BitAnd<Output = T>> ops::BitAnd for IntBitSet<T> {
    type Output = IntBitSet<T>;

    fn bitand(self, rhs: Self) -> Self::Output {
        IntBitSet::<T>(self.0 & rhs.0)
    }
}

impl<T: ops::BitAndAssign> ops::BitAndAssign for IntBitSet<T> {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl<T: PartialEq> PartialEq for IntBitSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
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
        let mut c = IntBitSet::<T>::empty();

        for i in iter {
            c |= IntBitSet::<T>::from_value(i);
        }

        c
    }
}
