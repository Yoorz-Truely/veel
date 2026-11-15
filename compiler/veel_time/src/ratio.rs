//! An exact rational over `i128`, always in lowest terms with a positive denominator.

use std::cmp::Ordering;
use std::fmt;

/// An exact rational number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ratio {
    num: i128,
    den: i128,
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Ratio {
    /// `num / den` in lowest terms, if `den` is not zero.
    pub fn new(num: i128, den: i128) -> Option<Ratio> {
        if den == 0 {
            return None;
        }
        let g = gcd(num, den).max(1);
        let sign = if den < 0 { -1 } else { 1 };
        Some(Ratio {
            num: sign * (num / g),
            den: sign * (den / g),
        })
    }

    /// A whole number.
    pub fn int(n: i128) -> Ratio {
        Ratio { num: n, den: 1 }
    }

    /// The numerator, in lowest terms.
    pub fn num(self) -> i128 {
        self.num
    }

    /// The denominator, in lowest terms, always positive.
    pub fn den(self) -> i128 {
        self.den
    }

    /// Whether it is a whole number.
    pub fn is_integer(self) -> bool {
        self.den == 1
    }

    /// The largest whole number not above it.
    pub fn floor(self) -> i128 {
        self.num.div_euclid(self.den)
    }

    /// The sum, if it fits.
    pub fn checked_add(self, o: Ratio) -> Option<Ratio> {
        let num = self
            .num
            .checked_mul(o.den)?
            .checked_add(o.num.checked_mul(self.den)?)?;
        Ratio::new(num, self.den.checked_mul(o.den)?)
    }

    /// The difference, if it fits.
    pub fn checked_sub(self, o: Ratio) -> Option<Ratio> {
        self.checked_add(Ratio {
            num: o.num.checked_neg()?,
            den: o.den,
        })
    }

    /// The product, if it fits.
    pub fn checked_mul(self, o: Ratio) -> Option<Ratio> {
        // Cross-reduce first, so exact results that fit are found.
        let g1 = gcd(self.num, o.den).max(1);
        let g2 = gcd(o.num, self.den).max(1);
        let num = (self.num / g1).checked_mul(o.num / g2)?;
        let den = (self.den / g2).checked_mul(o.den / g1)?;
        Ratio::new(num, den)
    }

    /// The quotient, if `o` is not zero and it fits.
    pub fn checked_div(self, o: Ratio) -> Option<Ratio> {
        if o.num == 0 {
            return None;
        }
        self.checked_mul(Ratio::new(o.den, o.num)?)
    }

    /// The exact decimal text, if the value has one (its denominator has only the factors 2 and
    /// 5): `1.5`, `-0.04`, `12`.
    pub fn to_decimal(self) -> Option<String> {
        let mut den = self.den;
        let (mut twos, mut fives) = (0u32, 0u32);
        while den % 2 == 0 {
            den /= 2;
            twos += 1;
        }
        while den % 5 == 0 {
            den /= 5;
            fives += 1;
        }
        if den != 1 {
            return None;
        }
        let places = twos.max(fives);
        let scaled = self
            .num
            .checked_mul(10i128.checked_pow(places)?)?
            .checked_div(self.den)?;
        Some(decimal_text(scaled, places))
    }

    /// The value to `places` decimals, cut towards zero, with `…` after it when that is not
    /// exact: for messages only, never for a stored value.
    pub fn approx(self, places: u32) -> String {
        let scale = 10i128.pow(places);
        let scaled = self.num.saturating_mul(scale) / self.den;
        let exact = self.num.saturating_mul(scale) % self.den == 0;
        let mut text = decimal_text(scaled, places);
        if exact {
            if text.contains('.') {
                text = text.trim_end_matches('0').trim_end_matches('.').to_string();
            }
        } else {
            if scaled == 0 && self.num < 0 {
                text.insert(0, '-');
            }
            text.push('…');
        }
        text
    }
}

fn decimal_text(scaled: i128, places: u32) -> String {
    let sign = if scaled < 0 { "-" } else { "" };
    let digits = scaled.unsigned_abs().to_string();
    if places == 0 {
        return format!("{sign}{digits}");
    }
    let places = places as usize;
    let digits = format!("{digits:0>width$}", width = places + 1);
    let (int, frac) = digits.split_at(digits.len() - places);
    format!("{sign}{int}.{frac}")
}

impl Ord for Ratio {
    fn cmp(&self, o: &Ratio) -> Ordering {
        // Denominators are positive; the products fit for the values Veel's words make.
        (self.num.saturating_mul(o.den)).cmp(&o.num.saturating_mul(self.den))
    }
}

impl PartialOrd for Ratio {
    fn partial_cmp(&self, o: &Ratio) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowest_terms_and_sign() {
        let r = Ratio::new(-30000, -1001).unwrap();
        assert_eq!((r.num(), r.den()), (30000, 1001));
        let r = Ratio::new(6, -4).unwrap();
        assert_eq!((r.num(), r.den()), (-3, 2));
        assert_eq!(Ratio::new(1, 0), None);
    }

    #[test]
    fn decimals() {
        assert_eq!(Ratio::new(3, 2).unwrap().to_decimal().unwrap(), "1.5");
        assert_eq!(Ratio::new(-1, 25).unwrap().to_decimal().unwrap(), "-0.04");
        assert_eq!(Ratio::int(12).to_decimal().unwrap(), "12");
        assert_eq!(Ratio::new(1, 3).unwrap().to_decimal(), None);
        assert_eq!(Ratio::new(288000, 1001).unwrap().approx(3), "287.712…");
        assert_eq!(Ratio::new(3, 4).unwrap().approx(3), "0.75");
        assert_eq!(Ratio::new(-1, 3000).unwrap().approx(3), "-0.000…");
    }
}
