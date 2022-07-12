use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, ShlAssign};
use std::{fmt, mem};

use crate::types::CellValue;

#[derive(Copy, Clone, Debug, PartialEq)]
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
    pub fn max() -> ValueSet {
        ValueSet(-1)
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

impl BitOr for ValueSet {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for ValueSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for ValueSet {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign for ValueSet {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for ValueSet {
    type Output = Self;

    fn not(self) -> Self::Output {
        ValueSet(!self.0)
    }
}

impl ShlAssign<usize> for ValueSet {
    fn shl_assign(&mut self, rhs: usize) {
        self.0 <<= rhs;
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
