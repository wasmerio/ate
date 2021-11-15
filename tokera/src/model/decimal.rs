use rust_decimal::Decimal as InnerDecimal;
use serde::de::{Unexpected, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::iter::Sum;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

#[allow(unused_imports)] // It's not actually dead code below, but the compiler thinks it is.
#[cfg(not(feature = "std"))]
use num_traits::float::FloatCore;
use num_traits::{
    CheckedAdd, CheckedDiv, CheckedMul, CheckedRem, CheckedSub, FromPrimitive, Num, One, Signed,
    ToPrimitive, Zero,
};

pub use rust_decimal::prelude::RoundingStrategy;

#[derive(Copy, Clone)]
pub struct Decimal(InnerDecimal);

impl Serialize for Decimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(DecimalVisitor)
    }
}

struct DecimalVisitor;

impl<'de> Visitor<'de> for DecimalVisitor {
    type Value = Decimal;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Decimal value")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        InnerDecimal::from_str(value)
            .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
            .map(Decimal)
    }
}

impl std::ops::Deref for Decimal {
    type Target = InnerDecimal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Decimal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Decimal {
    #[must_use]
    pub fn new(num: i64, scale: u32) -> Decimal {
        Decimal(InnerDecimal::new(num, scale))
    }

    #[must_use]
    pub fn from_i128_with_scale(num: i128, scale: u32) -> Decimal {
        Decimal(InnerDecimal::from_i128_with_scale(num, scale))
    }

    #[must_use]
    pub const fn from_parts(lo: u32, mid: u32, hi: u32, negative: bool, scale: u32) -> Decimal {
        Decimal(InnerDecimal::from_parts(lo, mid, hi, negative, scale))
    }

    pub fn from_scientific(value: &str) -> Result<Decimal, rust_decimal::Error> {
        Ok(Decimal(InnerDecimal::from_scientific(value)?))
    }

    #[must_use]
    pub fn round_dp(&self, dp: u32) -> Decimal {
        Decimal(self.0.round_dp(dp))
    }

    #[must_use]
    pub fn round_dp_with_strategy(&self, dp: u32, strategy: RoundingStrategy) -> Decimal {
        Decimal(self.0.round_dp_with_strategy(dp, strategy))
    }

    #[must_use]
    pub fn round(&self) -> Decimal {
        self.round_dp(0)
    }

    #[must_use]
    pub fn normalize(&self) -> Decimal {
        Decimal(self.0.normalize())
    }

    #[must_use]
    pub fn max(self, other: Decimal) -> Decimal {
        Decimal(self.0.max(other.0))
    }

    #[must_use]
    pub fn min(self, other: Decimal) -> Decimal {
        Decimal(self.0.min(other.0))
    }

    #[inline(always)]
    #[must_use]
    pub fn checked_add(self, other: Decimal) -> Option<Decimal> {
        self.0.checked_add(other.0).map(|a| Decimal(a))
    }

    #[inline(always)]
    #[must_use]
    pub fn checked_sub(self, other: Decimal) -> Option<Decimal> {
        self.0.checked_sub(other.0).map(|a| Decimal(a))
    }

    #[inline]
    #[must_use]
    pub fn checked_mul(self, other: Decimal) -> Option<Decimal> {
        self.0.checked_mul(other.0).map(|a| Decimal(a))
    }

    #[inline]
    #[must_use]
    pub fn checked_div(self, other: Decimal) -> Option<Decimal> {
        self.0.checked_div(other.0).map(|a| Decimal(a))
    }

    #[inline]
    #[must_use]
    pub fn checked_rem(self, other: Decimal) -> Option<Decimal> {
        self.0.checked_rem(other.0).map(|a| Decimal(a))
    }

    pub fn from_str_radix(str: &str, radix: u32) -> Result<Self, rust_decimal::Error> {
        Ok(Decimal(InnerDecimal::from_str_radix(str, radix)?))
    }

    #[must_use]
    pub const fn serialize(&self) -> [u8; 16] {
        self.0.serialize()
    }

    #[must_use]
    pub fn deserialize(bytes: [u8; 16]) -> Decimal {
        Decimal(InnerDecimal::deserialize(bytes))
    }
}

impl Default for Decimal {
    fn default() -> Self {
        Decimal::zero()
    }
}

impl Zero for Decimal {
    fn zero() -> Decimal {
        Decimal(InnerDecimal::zero())
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl One for Decimal {
    fn one() -> Decimal {
        Decimal(InnerDecimal::one())
    }
}

macro_rules! impl_from {
    ($T:ty, $from_ty:path) => {
        impl core::convert::From<$T> for Decimal {
            #[inline]
            fn from(t: $T) -> Self {
                $from_ty(t).unwrap()
            }
        }
    };
}
impl_from!(isize, FromPrimitive::from_isize);
impl_from!(i8, FromPrimitive::from_i8);
impl_from!(i16, FromPrimitive::from_i16);
impl_from!(i32, FromPrimitive::from_i32);
impl_from!(i64, FromPrimitive::from_i64);
impl_from!(usize, FromPrimitive::from_usize);
impl_from!(u8, FromPrimitive::from_u8);
impl_from!(u16, FromPrimitive::from_u16);
impl_from!(u32, FromPrimitive::from_u32);
impl_from!(u64, FromPrimitive::from_u64);

impl_from!(i128, FromPrimitive::from_i128);
impl_from!(u128, FromPrimitive::from_u128);

macro_rules! forward_val_val_binop {
    (impl $imp:ident for $res:ty, $method:ident) => {
        impl $imp<$res> for $res {
            type Output = $res;

            #[inline]
            fn $method(self, other: $res) -> $res {
                (&self).$method(&other)
            }
        }
    };
}

macro_rules! forward_ref_val_binop {
    (impl $imp:ident for $res:ty, $method:ident) => {
        impl<'a> $imp<$res> for &'a $res {
            type Output = $res;

            #[inline]
            fn $method(self, other: $res) -> $res {
                self.$method(&other)
            }
        }
    };
}

macro_rules! forward_val_ref_binop {
    (impl $imp:ident for $res:ty, $method:ident) => {
        impl<'a> $imp<&'a $res> for $res {
            type Output = $res;

            #[inline]
            fn $method(self, other: &$res) -> $res {
                (&self).$method(other)
            }
        }
    };
}

macro_rules! forward_all_binop {
    (impl $imp:ident for $res:ty, $method:ident) => {
        forward_val_val_binop!(impl $imp for $res, $method);
        forward_ref_val_binop!(impl $imp for $res, $method);
        forward_val_ref_binop!(impl $imp for $res, $method);
    };
}

impl Signed for Decimal {
    fn abs(&self) -> Self {
        Decimal(self.0.abs())
    }

    fn abs_sub(&self, other: &Self) -> Self {
        if self <= other {
            Decimal::zero()
        } else {
            Decimal(self.0.abs())
        }
    }

    fn signum(&self) -> Self {
        if self.is_zero() {
            Decimal::zero()
        } else {
            let mut value = Decimal::one();
            if self.is_sign_negative() {
                value.set_sign_negative(true);
            }
            value
        }
    }

    fn is_positive(&self) -> bool {
        self.0.is_sign_positive()
    }

    fn is_negative(&self) -> bool {
        self.0.is_sign_negative()
    }
}

impl CheckedAdd for Decimal {
    #[inline]
    fn checked_add(&self, v: &Decimal) -> Option<Decimal> {
        Decimal::checked_add(*self, *v)
    }
}

impl CheckedSub for Decimal {
    #[inline]
    fn checked_sub(&self, v: &Decimal) -> Option<Decimal> {
        Decimal::checked_sub(*self, *v)
    }
}

impl CheckedMul for Decimal {
    #[inline]
    fn checked_mul(&self, v: &Decimal) -> Option<Decimal> {
        Decimal::checked_mul(*self, *v)
    }
}

impl CheckedDiv for Decimal {
    #[inline]
    fn checked_div(&self, v: &Decimal) -> Option<Decimal> {
        Decimal::checked_div(*self, *v)
    }
}

impl CheckedRem for Decimal {
    #[inline]
    fn checked_rem(&self, v: &Decimal) -> Option<Decimal> {
        Decimal::checked_rem(*self, *v)
    }
}

impl Num for Decimal {
    type FromStrRadixErr = rust_decimal::Error;

    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        Decimal::from_str_radix(str, radix)
    }
}

impl FromStr for Decimal {
    type Err = rust_decimal::Error;

    fn from_str(value: &str) -> Result<Decimal, Self::Err> {
        Ok(Decimal(std::str::FromStr::from_str(value)?))
    }
}

impl FromPrimitive for Decimal {
    fn from_i32(n: i32) -> Option<Decimal> {
        InnerDecimal::from_i32(n).map(|a| Decimal(a))
    }

    fn from_i64(n: i64) -> Option<Decimal> {
        InnerDecimal::from_i64(n).map(|a| Decimal(a))
    }

    fn from_i128(n: i128) -> Option<Decimal> {
        InnerDecimal::from_i128(n).map(|a| Decimal(a))
    }

    fn from_u32(n: u32) -> Option<Decimal> {
        InnerDecimal::from_u32(n).map(|a| Decimal(a))
    }

    fn from_u64(n: u64) -> Option<Decimal> {
        InnerDecimal::from_u64(n).map(|a| Decimal(a))
    }

    fn from_u128(n: u128) -> Option<Decimal> {
        InnerDecimal::from_u128(n).map(|a| Decimal(a))
    }

    fn from_f32(n: f32) -> Option<Decimal> {
        InnerDecimal::from_f32(n).map(|a| Decimal(a))
    }

    fn from_f64(n: f64) -> Option<Decimal> {
        InnerDecimal::from_f64(n).map(|a| Decimal(a))
    }
}

impl ToPrimitive for Decimal {
    fn to_i64(&self) -> Option<i64> {
        self.0.to_i64()
    }

    fn to_i128(&self) -> Option<i128> {
        self.0.to_i128()
    }

    fn to_u64(&self) -> Option<u64> {
        self.0.to_u64()
    }

    fn to_u128(&self) -> Option<u128> {
        self.0.to_u128()
    }

    fn to_f64(&self) -> Option<f64> {
        self.0.to_f64()
    }
}

impl core::convert::TryFrom<f32> for Decimal {
    type Error = rust_decimal::Error;

    fn try_from(value: f32) -> Result<Self, rust_decimal::Error> {
        Ok(Decimal(InnerDecimal::try_from(value)?))
    }
}

impl core::convert::TryFrom<f64> for Decimal {
    type Error = rust_decimal::Error;

    fn try_from(value: f64) -> Result<Self, rust_decimal::Error> {
        Ok(Decimal(InnerDecimal::try_from(value)?))
    }
}

impl core::convert::TryFrom<Decimal> for f32 {
    type Error = rust_decimal::Error;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        core::convert::TryFrom::<InnerDecimal>::try_from(value.0)
    }
}

impl core::convert::TryFrom<Decimal> for f64 {
    type Error = rust_decimal::Error;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        core::convert::TryFrom::<InnerDecimal>::try_from(value.0)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Ok(self.0.fmt(f)?)
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Ok(self.0.fmt(f)?)
    }
}

impl fmt::LowerExp for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(self.0.fmt(f)?)
    }
}

impl fmt::UpperExp for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(self.0.fmt(f)?)
    }
}

impl Neg for Decimal {
    type Output = Decimal;

    fn neg(self) -> Decimal {
        Decimal(self.0.neg())
    }
}

impl<'a> Neg for &'a Decimal {
    type Output = Decimal;

    fn neg(self) -> Decimal {
        Decimal(self.0.neg())
    }
}

forward_all_binop!(impl Add for Decimal, add);

impl<'a, 'b> Add<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    #[inline(always)]
    fn add(self, other: &Decimal) -> Decimal {
        Decimal(self.0.add(&other.0))
    }
}

impl AddAssign for Decimal {
    fn add_assign(&mut self, other: Decimal) {
        self.0.add_assign(other.0)
    }
}

impl<'a> AddAssign<&'a Decimal> for Decimal {
    fn add_assign(&mut self, other: &'a Decimal) {
        self.0.add_assign(&other.0)
    }
}

impl<'a> AddAssign<Decimal> for &'a mut Decimal {
    fn add_assign(&mut self, other: Decimal) {
        self.0.add_assign(other.0)
    }
}

impl<'a> AddAssign<&'a Decimal> for &'a mut Decimal {
    fn add_assign(&mut self, other: &'a Decimal) {
        self.0.add_assign(&other.0)
    }
}

forward_all_binop!(impl Sub for Decimal, sub);

impl<'a, 'b> Sub<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    #[inline(always)]
    fn sub(self, other: &Decimal) -> Decimal {
        Decimal(self.0.sub(&other.0))
    }
}

impl SubAssign for Decimal {
    fn sub_assign(&mut self, other: Decimal) {
        self.0.sub_assign(other.0)
    }
}

impl<'a> SubAssign<&'a Decimal> for Decimal {
    fn sub_assign(&mut self, other: &'a Decimal) {
        self.0.sub_assign(&other.0)
    }
}

impl<'a> SubAssign<Decimal> for &'a mut Decimal {
    fn sub_assign(&mut self, other: Decimal) {
        self.0.sub_assign(&other.0)
    }
}

impl<'a> SubAssign<&'a Decimal> for &'a mut Decimal {
    fn sub_assign(&mut self, other: &'a Decimal) {
        self.0.sub_assign(&other.0)
    }
}

forward_all_binop!(impl Mul for Decimal, mul);

impl<'a, 'b> Mul<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    #[inline]
    fn mul(self, other: &Decimal) -> Decimal {
        Decimal(self.0.mul(&other.0))
    }
}

impl MulAssign for Decimal {
    fn mul_assign(&mut self, other: Decimal) {
        self.0.mul_assign(other.0)
    }
}

impl<'a> MulAssign<&'a Decimal> for Decimal {
    fn mul_assign(&mut self, other: &'a Decimal) {
        self.0.mul_assign(&other.0)
    }
}

impl<'a> MulAssign<Decimal> for &'a mut Decimal {
    fn mul_assign(&mut self, other: Decimal) {
        self.0.mul_assign(other.0)
    }
}

impl<'a> MulAssign<&'a Decimal> for &'a mut Decimal {
    fn mul_assign(&mut self, other: &'a Decimal) {
        self.0.mul_assign(&other.0)
    }
}

forward_all_binop!(impl Div for Decimal, div);

impl<'a, 'b> Div<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    fn div(self, other: &Decimal) -> Decimal {
        Decimal(self.0.div(&other.0))
    }
}

impl DivAssign for Decimal {
    fn div_assign(&mut self, other: Decimal) {
        self.0.div_assign(other.0)
    }
}

impl<'a> DivAssign<&'a Decimal> for Decimal {
    fn div_assign(&mut self, other: &'a Decimal) {
        self.0.div_assign(&other.0)
    }
}

impl<'a> DivAssign<Decimal> for &'a mut Decimal {
    fn div_assign(&mut self, other: Decimal) {
        self.0.div_assign(other.0)
    }
}

impl<'a> DivAssign<&'a Decimal> for &'a mut Decimal {
    fn div_assign(&mut self, other: &'a Decimal) {
        self.0.div_assign(&other.0)
    }
}

forward_all_binop!(impl Rem for Decimal, rem);

impl<'a, 'b> Rem<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    #[inline]
    fn rem(self, other: &Decimal) -> Decimal {
        Decimal(self.0.rem(&other.0))
    }
}

impl RemAssign for Decimal {
    fn rem_assign(&mut self, other: Decimal) {
        self.0.rem_assign(other.0)
    }
}

impl<'a> RemAssign<&'a Decimal> for Decimal {
    fn rem_assign(&mut self, other: &'a Decimal) {
        self.0.rem_assign(&other.0)
    }
}

impl<'a> RemAssign<Decimal> for &'a mut Decimal {
    fn rem_assign(&mut self, other: Decimal) {
        self.0.rem_assign(other.0)
    }
}

impl<'a> RemAssign<&'a Decimal> for &'a mut Decimal {
    fn rem_assign(&mut self, other: &'a Decimal) {
        self.0.rem_assign(&other.0)
    }
}

impl PartialEq for Decimal {
    #[inline]
    fn eq(&self, other: &Decimal) -> bool {
        self.0.cmp(&other.0) == Ordering::Equal
    }
}

impl Eq for Decimal {}

impl std::hash::Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, mut state: &mut H) {
        self.0.hash(&mut state);
    }
}

impl PartialOrd for Decimal {
    #[inline]
    fn partial_cmp(&self, other: &Decimal) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Decimal) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Sum for Decimal {
    fn sum<I: Iterator<Item = Decimal>>(iter: I) -> Self {
        let iter = iter.map(|a| a.0);
        Decimal(Sum::sum(iter))
    }
}

impl<'a> Sum<&'a Decimal> for Decimal {
    fn sum<I: Iterator<Item = &'a Decimal>>(iter: I) -> Self {
        let iter = iter.map(|a| &a.0);
        Decimal(Sum::sum(iter))
    }
}
