extern crate derive_more;

use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, ShlAssign};
use std::{fmt, mem};

use crate::types::CellValue;

#[derive(
    Copy, Clone, Debug, PartialEq, BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, ShlAssign,
)]
pub struct ValueSet(i64);

impl ValueSet {
    pub const BITS: u32 = (mem::size_of::<Self>() as u32) * u8::BITS;

    #[inline]
    pub fn from_value(value: CellValue) -> ValueSet {
        ValueSet(1 << (value - 1))
    }

    #[inline]
    pub fn from_value0(value: u32) -> ValueSet {
        ValueSet(1 << value)
    }

    #[inline]
    pub fn full(num_values: u32) -> ValueSet {
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
    pub fn value(&self) -> CellValue {
        self.0.trailing_zeros() + 1
    }

    #[inline]
    pub fn value0(&self) -> u32 {
        self.0.trailing_zeros()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn min_set(&self) -> ValueSet {
        ValueSet(self.0 & -self.0)
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
        let value = self.min_set();
        self.remove_set(value);
        Some(value)
    }
}

impl fmt::Display for ValueSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Iterator for ValueSet {
    type Item = ValueSet;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }

    #[inline]
    fn count(self) -> usize {
        self.0.count_ones() as usize
    }
}

impl FromIterator<u32> for ValueSet {
    fn from_iter<I: IntoIterator<Item = u32>>(iter: I) -> Self {
        let mut c = ValueSet::empty();

        for i in iter {
            c |= ValueSet::from_value0(i);
        }

        c
    }
}
