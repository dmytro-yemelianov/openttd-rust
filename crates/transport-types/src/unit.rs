use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub, Mul, Div, AddAssign, SubAssign};

/// Map coordinate in tiles
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Coord(pub i16); // Using signed for relative positioning

impl Coord {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(i16::MAX);
    pub const MIN: Self = Self(i16::MIN);
}

impl Add for Coord {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Sub for Coord {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Coord {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Coord {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

/// Simulation speed in tiles per tick
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Speed(pub f32);

impl Speed {
    pub const ZERO: Self = Self(0.0);
    pub const MAX: Self = Self(f32::MAX);
}

/// Money in smallest currency unit (cents)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Money(pub i64); // Using i64 for large amounts

impl Money {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(i64::MAX);
    pub const MIN: Self = Self(i64::MIN);
}

impl Add for Money {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Sub for Money {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Mul<i64> for Money {
    type Output = Self;
    fn mul(self, multiplier: i64) -> Self::Output {
        Self(self.0 * multiplier)
    }
}

impl Div<i64> for Money {
    type Output = Self;
    fn div(self, divisor: i64) -> Self::Output {
        Self(self.0 / divisor)
    }
}

/// Cargo amount in smallest unit (e.g., liters for fluid, units for discrete)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CargoAmount(pub u32);

impl CargoAmount {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(u32::MAX);
}

impl Add for CargoAmount {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Sub for CargoAmount {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl AddAssign for CargoAmount {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for CargoAmount {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

/// Temperature in hundredths of degrees Celsius
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Temperature(pub i32); // i32 to allow negative values

impl Temperature {
    pub const ZERO: Self = Self(0);
    pub const FREEZING: Self = Self(0); // 0.00°C
    pub const BOILING: Self = Self(100_00); // 100.00°C
}

impl Add for Temperature {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Sub for Temperature {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Temperature {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Temperature {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

/// Time in simulation ticks
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Ticks(pub u32);

impl Ticks {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(u32::MAX);
}

impl Add for Ticks {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Sub for Ticks {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Ticks {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Ticks {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

/// One-dimensional distance in tiles
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Distance(pub u32);

impl Distance {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(u32::MAX);
}

impl Add for Distance {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Sub for Distance {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Distance {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Distance {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

/// Cargo delivery payment in smallest currency unit
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Payment(pub i64);

impl Payment {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(i64::MAX);
    pub const MIN: Self = Self(i64::MIN);
}

impl Add for Payment {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Sub for Payment {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Payment {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Payment {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}