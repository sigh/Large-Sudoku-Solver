extern crate derive_more;

use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, ShlAssign};
use std::{fmt, mem};

#[derive(
    Copy, Clone, Debug, PartialEq, BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, ShlAssign,
)]
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
    pub fn value(&self) -> u8 {
        self.0.trailing_zeros() as u8
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
    pub fn pop(&mut self) -> Option<ValueSet> {
        if self.is_empty() {
            return None;
        }
        let min_set = ValueSet(self.0 & -self.0);
        self.remove_set(min_set);
        Some(min_set)
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
