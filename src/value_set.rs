use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::types::CellValue;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ValueSet(i64);

impl ValueSet {
    pub fn from_value(value: CellValue) -> ValueSet {
        ValueSet(1 << (value - 1))
    }

    pub fn from_value0(value: u32) -> ValueSet {
        ValueSet(1 << value)
    }

    pub fn full(num_values: u32) -> ValueSet {
        ValueSet((1 << num_values) - 1)
    }

    pub fn max() -> ValueSet {
        ValueSet(-1)
    }

    pub fn empty() -> ValueSet {
        ValueSet(0)
    }

    pub fn value(&self) -> CellValue {
        self.0.trailing_zeros() + 1
    }

    pub fn value0(&self) -> u32 {
        self.0.trailing_zeros()
    }

    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn min(&self) -> ValueSet {
        ValueSet(self.0 & -self.0)
    }

    pub fn remove(&mut self, other: ValueSet) {
        self.0 &= !other.0
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

impl fmt::Display for ValueSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
