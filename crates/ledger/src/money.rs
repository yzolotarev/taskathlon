//! Money in kopecks and calendar dates, both small and exact.

use serde::{Deserialize, Serialize};
use std::fmt;

/// An amount in kopecks. Arithmetic saturates so hostile input can't overflow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Money(pub i64);

impl Money {
    pub const ZERO: Money = Money(0);

    pub fn rub(r: i64) -> Money {
        Money(r.saturating_mul(100))
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn abs(self) -> Money {
        Money(self.0.saturating_abs())
    }

    /// `self * num / den`, rounded half away from zero to the kopeck.
    pub fn mul_div(self, num: i64, den: i64) -> Money {
        if den == 0 {
            return Money::ZERO;
        }
        let p = i128::from(self.0) * i128::from(num);
        let d = i128::from(den);
        let q = p / d;
        let r = p % d;
        let q = if r.abs() * 2 >= d.abs() { q + p.signum() * d.signum() } else { q };
        Money(i64::try_from(q).unwrap_or(if q > 0 { i64::MAX } else { i64::MIN }))
    }

    /// Parses "1 234,56", "1234.5" or "-10".
    pub fn parse(s: &str) -> Option<Money> {
        let t: String = s.chars().filter(|c| !c.is_whitespace() && *c != '\u{a0}').collect::<String>().replace(',', ".");
        if t.is_empty() {
            return None;
        }
        let (neg, t) = match t.strip_prefix('-') {
            Some(rest) => (true, rest.to_string()),
            None => (false, t),
        };
        let (int, frac) = t.split_once('.').unwrap_or((&t, ""));
        if int.is_empty() && frac.is_empty()
            || !int.chars().all(|c| c.is_ascii_digit())
            || !frac.chars().all(|c| c.is_ascii_digit())
            || frac.len() > 2
        {
            return None;
        }
        let rub: i64 = if int.is_empty() { 0 } else { int.parse().ok()? };
        let kop: i64 = match frac.len() {
            0 => 0,
            1 => frac.parse::<i64>().ok()? * 10,
            _ => frac.parse().ok()?,
        };
        let v = rub.checked_mul(100)?.checked_add(kop)?;
        Some(Money(if neg { -v } else { v }))
    }
}

impl std::ops::Add for Money {
    type Output = Money;
    fn add(self, o: Money) -> Money {
        Money(self.0.saturating_add(o.0))
    }
}

impl std::ops::Sub for Money {
    type Output = Money;
    fn sub(self, o: Money) -> Money {
        Money(self.0.saturating_sub(o.0))
    }
}

impl std::ops::AddAssign for Money {
    fn add_assign(&mut self, o: Money) {
        *self = *self + o;
    }
}

impl std::ops::SubAssign for Money {
    fn sub_assign(&mut self, o: Money) {
        *self = *self - o;
    }
}

impl std::ops::Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(self.0.saturating_neg())
    }
}

impl std::iter::Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, |a, b| a + b)
    }
}

/// "1 234,56" with a thin group separator, as Russian reports print money.
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let neg = self.0 < 0;
        let v = self.0.unsigned_abs();
        let rub = (v / 100).to_string();
        let mut grouped = String::new();
        for (i, c) in rub.chars().enumerate() {
            if i > 0 && (rub.len() - i).is_multiple_of(3) {
                grouped.push(' ');
            }
            grouped.push(c);
        }
        write!(f, "{}{grouped},{:02}", if neg { "-" } else { "" }, v % 100)
    }
}

/// A calendar date. Ordering is chronological.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Date {
    pub y: u16,
    pub m: u8,
    pub d: u8,
}

impl Date {
    pub fn new(y: u16, m: u8, d: u8) -> Option<Date> {
        (1..=12).contains(&m).then_some(())?;
        (d >= 1 && d <= days_in_month(y, m)).then_some(Date { y, m, d })
    }

    /// Accepts "2026-03-15" and "15.03.2026".
    pub fn parse(s: &str) -> Option<Date> {
        let s = s.trim();
        if let Some((y, rest)) = s.split_once('-') {
            let (m, d) = rest.split_once('-')?;
            return Date::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
        }
        let mut it = s.split('.');
        let d = it.next()?.parse().ok()?;
        let m = it.next()?.parse().ok()?;
        let y = it.next()?.parse().ok()?;
        Date::new(y, m, d)
    }

    pub fn month_start(self) -> Date {
        Date { d: 1, ..self }
    }

    pub fn month_end(self) -> Date {
        Date { d: days_in_month(self.y, self.m), ..self }
    }

    /// Months from `self`'s month to `other`'s (negative when earlier).
    pub fn months_until(self, other: Date) -> i32 {
        (i32::from(other.y) - i32::from(self.y)) * 12 + i32::from(other.m) - i32::from(self.m)
    }

    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}.{:02}.{:04}", self.d, self.m, self.y)
    }
}

pub fn days_in_month(y: u16, m: u8) -> u8 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400) => 29,
        2 => 28,
        _ => 0,
    }
}

pub const MONTHS: [&str; 12] = ["январь", "февраль", "март", "апрель", "май", "июнь", "июль", "август", "сентябрь", "октябрь", "ноябрь", "декабрь"];
