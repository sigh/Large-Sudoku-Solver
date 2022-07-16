extern crate derive_more;

use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use std::{fmt, mem};

#[derive(Copy, Clone, Debug, PartialEq, BitAnd, BitAndAssign, BitOr, BitOrAssign, Not)]
pub struct ValueSet(i64);

impl ValueSet {
    pub const BITS: u8 = (mem::size_of::<Self>() as u8) * (u8::BITS as u8);

    #[inline]
    pub fn from_value(value: u8) -> ValueSet {
        ValueSet(1 << value)
    }

    #[inline]
    pub fn full(num_values: u8) -> ValueSet {
        ValueSet(if num_values == Self::BITS {
            -1
        } else {
            !(-1 << num_values)
        })
    }

    #[inline]
    pub fn empty() -> ValueSet {
        ValueSet(0)
    }

    #[inline]
    pub fn value(&self) -> Option<u8> {
        if self.count() != 1 {
            return None;
        }
        self.min()
    }

    #[inline]
    pub fn min(&self) -> Option<u8> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.trailing_zeros() as u8)
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    #[inline]
    pub fn remove_set(&mut self, other: ValueSet) {
        self.0 &= !other.0
    }

    #[inline]
    pub fn pop(&mut self) -> Option<u8> {
        let value = self.min()?;
        self.remove_set(ValueSet::from_value(value));
        Some(value)
    }
}

impl fmt::Display for ValueSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromIterator<u8> for ValueSet {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        let mut c = ValueSet::empty();

        for i in iter {
            c |= ValueSet::from_value(i);
        }

        c
    }
}
