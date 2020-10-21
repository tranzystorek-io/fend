use crate::err::{IntErr, Interrupt, Never};
use crate::interrupt::test_int;
use crate::num::biguint::BigUint;
use crate::num::Exact;
use crate::num::{Base, DivideByZero, FormattingStyle};
use std::cmp::Ordering;
use std::fmt;
use std::ops::Neg;

mod sign {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Sign {
        Positive,
        Negative,
    }

    impl Sign {
        pub const fn flip(self) -> Self {
            match self {
                Self::Positive => Self::Negative,
                Self::Negative => Self::Positive,
            }
        }

        pub const fn sign_of_product(a: Self, b: Self) -> Self {
            match (a, b) {
                (Self::Positive, Self::Positive) | (Self::Negative, Self::Negative) => {
                    Self::Positive
                }
                (Self::Positive, Self::Negative) | (Self::Negative, Self::Positive) => {
                    Self::Negative
                }
            }
        }
    }
}

use super::biguint::FormattedBigUint;
use sign::Sign;

#[derive(Clone)]
pub struct BigRat {
    sign: Sign,
    num: BigUint,
    den: BigUint,
}

impl fmt::Debug for BigRat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.sign == Sign::Negative {
            write!(f, "-")?;
        }
        write!(f, "{:?}/{:?}", self.num, self.den)
    }
}

impl Ord for BigRat {
    fn cmp(&self, other: &Self) -> Ordering {
        let int = &crate::interrupt::Never::default();
        let diff = self.clone().add(-other.clone(), int).unwrap();
        if diff.num == 0.into() {
            Ordering::Equal
        } else if diff.sign == Sign::Positive {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}

impl PartialOrd for BigRat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BigRat {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for BigRat {}

impl BigRat {
    pub fn try_as_usize<I: Interrupt>(mut self, int: &I) -> Result<usize, IntErr<String, I>> {
        if self.sign == Sign::Negative && self.num != 0.into() {
            return Err("Negative numbers are not allowed".to_string())?;
        }
        self = self.simplify(int)?;
        if self.den != 1.into() {
            return Err("Cannot convert fraction to integer".to_string())?;
        }
        Ok(self.num.try_as_usize().map_err(|e| e.to_string())?)
    }

    #[allow(clippy::float_arithmetic)]
    pub fn into_f64<I: Interrupt>(mut self, int: &I) -> Result<f64, IntErr<Never, I>> {
        self = self.simplify(int)?;
        let positive_result = self.num.as_f64() / self.den.as_f64();
        if self.sign == Sign::Positive {
            Ok(positive_result)
        } else {
            Ok(-positive_result)
        }
    }

    #[allow(
        clippy::as_conversions,
        clippy::float_arithmetic,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn from_f64<I: Interrupt>(mut f: f64, int: &I) -> Result<Self, IntErr<Never, I>> {
        let negative = f < 0.0;
        if negative {
            f = -f;
        }
        let i = (f * u64::MAX as f64) as u128;
        let part1 = i as u64;
        let part2 = (i >> 64) as u64;
        Ok(Self {
            sign: if negative {
                Sign::Negative
            } else {
                Sign::Positive
            },
            num: BigUint::from(part1)
                .add(&BigUint::from(part2).mul(&BigUint::from(u64::MAX), int)?),
            den: BigUint::from(u64::MAX),
        })
    }

    // sin works for all real numbers
    pub fn sin<I: Interrupt>(self, int: &I) -> Result<Exact<Self>, IntErr<Never, I>> {
        Ok(if self == 0.into() {
            Exact::new(Self::from(0), true)
        } else {
            Exact::new(Self::from_f64(f64::sin(self.into_f64(int)?), int)?, false)
        })
    }

    // asin, acos and atan only work for values between -1 and 1
    pub fn asin<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<String, I>> {
        let one: Self = 1.into();
        if self > one || self < -one {
            return Err("Value must be between -1 and 1".to_string())?;
        }
        Ok(Self::from_f64(f64::asin(self.into_f64(int)?), int)?)
    }

    pub fn acos<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<String, I>> {
        let one: Self = 1.into();
        if self > one || self < -one {
            return Err("Value must be between -1 and 1".to_string())?;
        }
        Ok(Self::from_f64(f64::acos(self.into_f64(int)?), int)?)
    }

    // note that this works for any real number, unlike asin and acos
    pub fn atan<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<Never, I>> {
        Ok(Self::from_f64(f64::atan(self.into_f64(int)?), int)?)
    }

    pub fn sinh<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<Never, I>> {
        Ok(Self::from_f64(f64::sinh(self.into_f64(int)?), int)?)
    }

    pub fn cosh<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<Never, I>> {
        Ok(Self::from_f64(f64::cosh(self.into_f64(int)?), int)?)
    }

    pub fn tanh<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<Never, I>> {
        Ok(Self::from_f64(f64::tanh(self.into_f64(int)?), int)?)
    }

    pub fn asinh<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<Never, I>> {
        Ok(Self::from_f64(f64::asinh(self.into_f64(int)?), int)?)
    }

    // value must not be less than 1
    pub fn acosh<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<String, I>> {
        if self < 1.into() {
            return Err("Value must not be less than 1".to_string())?;
        }
        Ok(Self::from_f64(f64::acosh(self.into_f64(int)?), int)?)
    }

    // value must be between -1 and 1.
    pub fn atanh<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<String, I>> {
        let one: Self = 1.into();
        if self >= one || self <= -one {
            return Err("Value must be between -1 and 1".to_string())?;
        }
        Ok(Self::from_f64(f64::atanh(self.into_f64(int)?), int)?)
    }

    // For all logs: value must be greater than 0
    pub fn ln<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<String, I>> {
        if self <= 0.into() {
            return Err("Value must be greater than 0".to_string())?;
        }
        Ok(Self::from_f64(f64::ln(self.into_f64(int)?), int)?)
    }

    pub fn log2<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<String, I>> {
        if self <= 0.into() {
            return Err("Value must be greater than 0".to_string())?;
        }
        Ok(Self::from_f64(f64::log2(self.into_f64(int)?), int)?)
    }

    pub fn log10<I: Interrupt>(self, int: &I) -> Result<Self, IntErr<String, I>> {
        if self <= 0.into() {
            return Err("Value must be greater than 0".to_string())?;
        }
        Ok(Self::from_f64(f64::log10(self.into_f64(int)?), int)?)
    }

    pub fn factorial<I: Interrupt>(mut self, int: &I) -> Result<Self, IntErr<String, I>> {
        self = self.simplify(int)?;
        if self.den != 1.into() {
            return Err("Factorial is only supported for integers".to_string())?;
        }
        if self.sign == Sign::Negative && self.num != 0.into() {
            return Err("Factorial is only supported for positive integers".to_string())?;
        }
        Ok(Self {
            sign: Sign::Positive,
            num: self.num.factorial(int)?,
            den: self.den,
        })
    }

    /// compute a + b
    fn add_internal<I: Interrupt>(self, rhs: Self, int: &I) -> Result<Self, IntErr<Never, I>> {
        // a + b == -((-a) + (-b))
        if self.sign == Sign::Negative {
            return Ok(-((-self).add_internal(-rhs, int)?));
        }

        assert_eq!(self.sign, Sign::Positive);

        Ok(if self.den == rhs.den {
            if rhs.sign == Sign::Negative && self.num < rhs.num {
                Self {
                    sign: Sign::Negative,
                    num: rhs.num.sub(&self.num),
                    den: self.den,
                }
            } else {
                Self {
                    sign: Sign::Positive,
                    num: if rhs.sign == Sign::Positive {
                        self.num.add(&rhs.num)
                    } else {
                        self.num.sub(&rhs.num)
                    },
                    den: self.den,
                }
            }
        } else {
            let gcd = BigUint::gcd(self.den.clone(), rhs.den.clone(), int)?;
            let new_denominator = self
                .den
                .clone()
                .mul(&rhs.den, int)?
                .div(&gcd, int)
                .map_err(IntErr::unwrap)?;
            let a = self
                .num
                .mul(&rhs.den, int)?
                .div(&gcd, int)
                .map_err(IntErr::unwrap)?;
            let b = rhs
                .num
                .mul(&self.den, int)?
                .div(&gcd, int)
                .map_err(IntErr::unwrap)?;

            if rhs.sign == Sign::Negative && a < b {
                Self {
                    sign: Sign::Negative,
                    num: b.sub(&a),
                    den: new_denominator,
                }
            } else {
                Self {
                    sign: Sign::Positive,
                    num: if rhs.sign == Sign::Positive {
                        a.add(&b)
                    } else {
                        a.sub(&b)
                    },
                    den: new_denominator,
                }
            }
        })
    }

    fn simplify<I: Interrupt>(mut self, int: &I) -> Result<Self, IntErr<Never, I>> {
        if self.den == 1.into() {
            return Ok(self);
        }
        let gcd = BigUint::gcd(self.num.clone(), self.den.clone(), int)?;
        self.num = self.num.div(&gcd, int).map_err(IntErr::unwrap)?;
        self.den = self.den.div(&gcd, int).map_err(IntErr::unwrap)?;
        Ok(self)
    }

    pub fn div<I: Interrupt>(self, rhs: &Self, int: &I) -> Result<Self, IntErr<DivideByZero, I>> {
        if rhs.num == 0.into() {
            return Err(DivideByZero {})?;
        }
        Ok(Self {
            sign: Sign::sign_of_product(self.sign, rhs.sign),
            num: self.num.mul(&rhs.den, int)?,
            den: self.den.mul(&rhs.num, int)?,
        })
    }

    // test if this fraction has a terminating representation
    // e.g. in base 10: 1/4 = 0.25, but not 1/3
    fn terminates_in_base<I: Interrupt>(
        &self,
        base: Base,
        int: &I,
    ) -> Result<bool, IntErr<Never, I>> {
        let mut x = self.clone();
        let base_as_u64: u64 = base.base_as_u8().into();
        let base = Self {
            sign: Sign::Positive,
            num: base_as_u64.into(),
            den: 1.into(),
        };
        loop {
            let old_den = x.den.clone();
            x = x.mul(&base, int)?.simplify(int)?;
            let new_den = x.den.clone();
            if new_den == old_den {
                break;
            }
        }
        Ok(x.den == 1.into())
    }

    fn format_as_integer<I: Interrupt>(
        num: &BigUint,
        base: Base,
        sign: Sign,
        imag: bool,
        use_parens_if_product: bool,
        int: &I,
    ) -> Result<Exact<FormattedBigRat>, IntErr<Never, I>> {
        let ty = if imag && !base.has_prefix() && num == &1.into() {
            FormattedBigRatType::Integer(None, "i", false)
        } else {
            FormattedBigRatType::Integer(
                Some(num.format(base, true, int)?),
                if imag {
                    if base.base_as_u8() >= 19 {
                        // at this point 'i' could be a digit,
                        // so we need a space to disambiguate
                        " i"
                    } else {
                        "i"
                    }
                } else {
                    ""
                },
                // print surrounding parentheses if the number is imaginary
                use_parens_if_product && imag,
            )
        };
        Ok(Exact::new(FormattedBigRat { sign, ty }, true))
    }

    fn format_as_fraction<I: Interrupt>(
        num: &BigUint,
        den: &BigUint,
        base: Base,
        sign: Sign,
        imag: bool,
        mixed: bool,
        use_parens: bool,
        int: &I,
    ) -> Result<Exact<FormattedBigRat>, IntErr<Never, I>> {
        let formatted_den = den.format(base, true, int)?;
        let (pref, num) = if mixed {
            let (prefix, num) = num.divmod(den, int).map_err(IntErr::unwrap)?;
            if prefix == 0.into() {
                (None, num)
            } else {
                (Some(prefix.format(base, true, int)?), num)
            }
        } else {
            (None, num.clone())
        };
        // mixed fractions without a prefix aren't really mixed
        let actually_mixed = pref.is_some();
        Ok(Exact::new(
            FormattedBigRat {
                sign,
                ty: if imag && !actually_mixed && !base.has_prefix() && num == 1.into() {
                    FormattedBigRatType::Fraction(pref, None, "i", formatted_den, "", use_parens)
                } else {
                    let formatted_num = num.format(base, true, int)?;
                    let i_suffix = if imag {
                        if base.base_as_u8() >= 19 || actually_mixed {
                            " i"
                        } else {
                            "i"
                        }
                    } else {
                        ""
                    };
                    let (isuf1, isuf2) = if actually_mixed {
                        ("", i_suffix)
                    } else {
                        (i_suffix, "")
                    };
                    FormattedBigRatType::Fraction(
                        pref,
                        Some(formatted_num),
                        isuf1,
                        formatted_den,
                        isuf2,
                        use_parens,
                    )
                },
            },
            true,
        ))
    }

    // Formats as an integer if possible, or a terminating float, otherwise as
    // either a fraction or a potentially approximated floating-point number.
    // The result 'exact' field indicates whether the number was exact or not.
    pub fn format<I: Interrupt>(
        &self,
        base: Base,
        style: FormattingStyle,
        imag: bool,
        use_parens_if_fraction: bool,
        int: &I,
    ) -> Result<Exact<FormattedBigRat>, IntErr<Never, I>> {
        let mut x = self.clone().simplify(int)?;
        let sign = if x.sign == Sign::Positive || x == 0.into() {
            Sign::Positive
        } else {
            Sign::Negative
        };
        x.sign = Sign::Positive;

        // try as integer if possible
        if x.den == 1.into() {
            return Ok(Self::format_as_integer(
                &x.num,
                base,
                sign,
                imag,
                use_parens_if_fraction,
                int,
            )?);
        }

        let mut terminating_res = None;
        let mut terminating = || match terminating_res {
            None => {
                let t = x.terminates_in_base(base, int)?;
                terminating_res = Some(t);
                Ok(t)
            }
            Some(t) => Ok(t),
        };
        let fraction = style == FormattingStyle::ExactFraction
            || style == FormattingStyle::MixedFraction
            || (style == FormattingStyle::ExactFloatWithFractionFallback && !terminating()?);
        if fraction {
            let mixed = style == FormattingStyle::MixedFraction
                || style == FormattingStyle::ExactFloatWithFractionFallback;
            return Ok(Self::format_as_fraction(
                &x.num,
                &x.den,
                base,
                sign,
                imag,
                mixed,
                use_parens_if_fraction,
                int,
            )?);
        }

        // not a fraction, will be printed as a decimal
        let num_trailing_digits_to_print = if style == FormattingStyle::ExactFloat
            || (style == FormattingStyle::ExactFloatWithFractionFallback && terminating()?)
        {
            None
        } else if let FormattingStyle::DecimalPlaces(n) = style {
            Some(n)
        } else {
            Some(10)
        };
        let integer_part = x.num.clone().div(&x.den, int).map_err(IntErr::unwrap)?;
        let print_integer_part = |ignore_minus_if_zero: bool| {
            let mut res = String::new();
            if sign == Sign::Negative && (!ignore_minus_if_zero || integer_part != 0.into()) {
                res.push('-');
            }
            res.push_str(&integer_part.format(base, true, int)?.to_string());
            Ok(res)
        };
        let integer_as_rational = Self {
            sign: Sign::Positive,
            num: integer_part.clone(),
            den: 1.into(),
        };
        let remaining_fraction = x.clone().add(-integer_as_rational, int)?;
        let mut formatted_trailing_digits = Self::format_trailing_digits(
            base,
            &remaining_fraction.num,
            &remaining_fraction.den,
            num_trailing_digits_to_print,
            terminating,
            print_integer_part,
            int,
        )?;
        if imag {
            if base.base_as_u8() >= 19 {
                formatted_trailing_digits.value.push(' ');
            }
            formatted_trailing_digits.value.push('i');
        }
        Ok(Exact::new(
            FormattedBigRat {
                sign: Sign::Positive,
                ty: FormattedBigRatType::Decimal(formatted_trailing_digits.value),
            },
            formatted_trailing_digits.exact,
        ))
    }

    /// Prints the decimal expansion of num/den, where num < den, in the given base.
    /// Behaviour:
    ///   * If `max_digits` is None, print the whole decimal. Recurring digits are indicated
    ///     with parentheses.
    ///   * If `max_digits` is given, only up to that many digits are printed, and recurring
    ///     digits will not printed in parentheses but will instead be repeated up to `max_digits`.
    ///     Any trailing zeroes are not printed.
    fn format_trailing_digits<I: Interrupt>(
        base: Base,
        numerator: &BigUint,
        denominator: &BigUint,
        max_digits: Option<usize>,
        mut terminating: impl FnMut() -> Result<bool, IntErr<Never, I>>,
        print_integer_part: impl Fn(bool) -> Result<String, IntErr<Never, I>>,
        int: &I,
    ) -> Result<Exact<String>, IntErr<Never, I>> {
        enum NextDigitErr<I: Interrupt> {
            Interrupt(IntErr<Never, I>),
            Terminated,
        };
        impl<I: Interrupt> From<IntErr<Never, I>> for NextDigitErr<I> {
            fn from(i: IntErr<Never, I>) -> Self {
                Self::Interrupt(i)
            }
        }

        let mut trailing_digits = String::new();

        let base_as_u64: u64 = base.base_as_u8().into();
        let b: BigUint = base_as_u64.into();
        let next_digit = |i: usize,
                          num: BigUint,
                          base: &BigUint|
         -> Result<(BigUint, BigUint), NextDigitErr<I>> {
            test_int(int)?;
            if num == 0.into() || max_digits == Some(i) {
                return Err(NextDigitErr::Terminated);
            }
            // digit = base * numerator / denominator
            // next_numerator = base * numerator - digit * denominator
            let bnum = num.mul(base, int)?;
            let digit = bnum
                .clone()
                .div(&denominator, int)
                .map_err(IntErr::unwrap)?;
            let next_num = bnum.sub(&digit.clone().mul(&denominator, int)?);
            Ok((next_num, digit))
        };
        let fold_digits = |mut s: String, digit: BigUint| -> Result<String, IntErr<Never, I>> {
            let digit_str = digit.format(base, false, int)?.to_string();
            s.push_str(digit_str.as_str());
            Ok(s)
        };
        let skip_cycle_detection = max_digits.is_some() || terminating()?;
        if skip_cycle_detection {
            let mut current_numerator = numerator.clone();
            let mut i = 0;
            let mut trailing_zeroes = 0;
            let mut wrote_decimal_point = false;
            loop {
                match next_digit(i, current_numerator.clone(), &b) {
                    Ok((next_n, digit)) => {
                        current_numerator = next_n;
                        if digit == 0.into() {
                            trailing_zeroes += 1;
                        } else {
                            if !wrote_decimal_point {
                                // always print leading minus because we have non-zero digits
                                trailing_digits.push_str(&print_integer_part(false)?);
                                trailing_digits.push('.');
                                wrote_decimal_point = true;
                            }
                            for _ in 0..trailing_zeroes {
                                trailing_digits.push('0');
                            }
                            trailing_zeroes = 0;
                            trailing_digits.push_str(&digit.format(base, false, int)?.to_string());
                        }
                    }
                    Err(NextDigitErr::Terminated) => {
                        if !wrote_decimal_point {
                            // if we reach this point we haven't printed any non-zero digits,
                            // so we can skip the leading minus sign if the integer part is also zero
                            trailing_digits.push_str(&print_integer_part(true)?);
                        }
                        // is the number exact, or did we need to truncate?
                        let exact = current_numerator == 0.into();
                        return Ok(Exact::new(trailing_digits, exact));
                    }
                    Err(NextDigitErr::Interrupt(i)) => {
                        return Err(i);
                    }
                }
                i += 1;
            }
        }
        match Self::brents_algorithm(
            next_digit,
            fold_digits,
            numerator.clone(),
            &b,
            String::new(),
        ) {
            Ok((cycle_length, location, output)) => {
                let (ab, _) = output.split_at(location + cycle_length);
                let (a, b) = ab.split_at(location);
                // we're printing non-zero digits, so always include minus sign
                trailing_digits.push_str(&print_integer_part(false)?);
                trailing_digits.push('.');
                trailing_digits.push_str(a);
                trailing_digits.push('(');
                trailing_digits.push_str(b);
                trailing_digits.push(')');
            }
            Err(NextDigitErr::Terminated) => {
                panic!("Decimal number terminated unexpectedly");
            }
            Err(NextDigitErr::Interrupt(i)) => {
                return Err(i);
            }
        }
        Ok(Exact::new(trailing_digits, true)) // the recurring decimal is exact
    }

    // Brent's cycle detection algorithm (based on pseudocode from Wikipedia)
    // returns (length of cycle, index of first element of cycle, collected result)
    fn brents_algorithm<T: Clone + Eq, R, U, E1: From<E2>, E2>(
        f: impl Fn(usize, T, &T) -> Result<(T, U), E1>,
        g: impl Fn(R, U) -> Result<R, E2>,
        x0: T,
        state: &T,
        r0: R,
    ) -> Result<(usize, usize, R), E1> {
        // main phase: search successive powers of two
        let mut power = 1;
        // lam is the length of the cycle
        let mut lam = 1;
        let mut tortoise = x0.clone();
        let mut depth = 0;
        let (mut hare, _) = f(depth, x0.clone(), state)?;
        depth += 1;
        while tortoise != hare {
            if power == lam {
                tortoise = hare.clone();
                power *= 2;
                lam = 0;
            }
            hare = f(depth, hare, state)?.0;
            depth += 1;
            lam += 1;
        }

        // Find the position of the first repetition of length lam
        tortoise = x0.clone();
        hare = x0;
        let mut collected_res = r0;
        let mut hare_depth = 0;
        for _ in 0..lam {
            let (new_hare, u) = f(hare_depth, hare, state)?;
            hare_depth += 1;
            hare = new_hare;
            collected_res = g(collected_res, u)?;
        }
        // The distance between the hare and tortoise is now lam.

        // Next, the hare and tortoise move at same speed until they agree
        // mu will be the length of the initial sequence, before the cycle
        let mut mu = 0;
        let mut tortoise_depth = 0;
        while tortoise != hare {
            tortoise = f(tortoise_depth, tortoise, state)?.0;
            tortoise_depth += 1;
            let (new_hare, u) = f(hare_depth, hare, state)?;
            hare_depth += 1;
            hare = new_hare;
            collected_res = g(collected_res, u)?;
            mu += 1;
        }
        Ok((lam, mu, collected_res))
    }

    pub fn pow<I: Interrupt>(
        mut self,
        mut rhs: Self,
        int: &I,
    ) -> Result<Exact<Self>, IntErr<String, I>> {
        self = self.simplify(int)?;
        rhs = rhs.simplify(int)?;
        if self.num != 0.into() && self.sign == Sign::Negative && rhs.den != 1.into() {
            return Err("Roots of negative numbers are not supported".to_string())?;
        }
        if rhs.sign == Sign::Negative {
            // a^-b => 1/a^b
            rhs.sign = Sign::Positive;
            let inverse_res = self.pow(rhs, int)?;
            return Ok(Exact::new(
                Self::from(1)
                    .div(&inverse_res.value, int)
                    .map_err(IntErr::into_string)?,
                inverse_res.exact,
            ));
        }
        let result_sign =
            if self.sign == Sign::Positive || rhs.num.is_even(int).map_err(IntErr::into_string)? {
                Sign::Positive
            } else {
                Sign::Negative
            };
        let pow_res = Self {
            sign: result_sign,
            num: BigUint::pow(&self.num, &rhs.num, int).map_err(IntErr::into_string)?,
            den: BigUint::pow(&self.den, &rhs.num, int).map_err(IntErr::into_string)?,
        };
        if rhs.den == 1.into() {
            Ok(Exact::new(pow_res, true))
        } else {
            Ok(pow_res.root_n(
                &Self {
                    sign: Sign::Positive,
                    num: rhs.den,
                    den: 1.into(),
                },
                int,
            )?)
        }
    }

    /// n must be an integer
    fn iter_root_n<I: Interrupt>(
        mut low_bound: Self,
        val: &Self,
        n: &Self,
        int: &I,
    ) -> Result<Self, IntErr<String, I>> {
        let mut high_bound = low_bound.clone().add(1.into(), int)?;
        for _ in 0..30 {
            let guess = low_bound
                .clone()
                .add(high_bound.clone(), int)?
                .div(&2.into(), int)
                .map_err(IntErr::into_string)?;
            if &guess.clone().pow(n.clone(), int)?.value < val {
                low_bound = guess;
            } else {
                high_bound = guess;
            }
        }
        Ok(low_bound
            .add(high_bound, int)?
            .div(&2.into(), int)
            .map_err(IntErr::into_string)?)
    }

    // the boolean indicates whether or not the result is exact
    // n must be an integer
    pub fn root_n<I: Interrupt>(self, n: &Self, int: &I) -> Result<Exact<Self>, IntErr<String, I>> {
        if self.num != 0.into() && self.sign == Sign::Negative {
            return Err("Can't compute roots of negative numbers".to_string())?;
        }
        let n = n.clone().simplify(int)?;
        if n.den != 1.into() || n.sign == Sign::Negative {
            return Err("Can't compute non-integer or negative roots".to_string())?;
        }
        let n = &n.num;
        if self.num == 0.into() {
            return Ok(Exact::new(self, true));
        }
        let num = self
            .clone()
            .num
            .root_n(n, int)
            .map_err(IntErr::into_string)?;
        let den = self
            .clone()
            .den
            .root_n(n, int)
            .map_err(IntErr::into_string)?;
        if num.exact && den.exact {
            return Ok(Exact::new(
                Self {
                    sign: Sign::Positive,
                    num: num.value,
                    den: den.value,
                },
                true,
            ));
        }
        // TODO check in which cases this might still be exact
        let num_rat = if num.exact {
            Self::from(num.value)
        } else {
            Self::iter_root_n(
                Self::from(num.value),
                &Self::from(self.num),
                &Self::from(n.clone()),
                int,
            )?
        };
        let den_rat = if den.exact {
            Self::from(den.value)
        } else {
            Self::iter_root_n(
                Self::from(den.value),
                &Self::from(self.den),
                &Self::from(n.clone()),
                int,
            )?
        };
        Ok(Exact::new(
            num_rat.div(&den_rat, int).map_err(IntErr::into_string)?,
            false,
        ))
    }

    pub fn mul<I: Interrupt>(self, rhs: &Self, int: &I) -> Result<Self, IntErr<Never, I>> {
        Ok(Self {
            sign: Sign::sign_of_product(self.sign, rhs.sign),
            num: self.num.mul(&rhs.num, int)?,
            den: self.den.mul(&rhs.den, int)?,
        })
    }

    pub fn add<I: Interrupt>(self, rhs: Self, int: &I) -> Result<Self, IntErr<Never, I>> {
        Ok(self.add_internal(rhs, int)?)
    }
}

impl Neg for BigRat {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            sign: self.sign.flip(),
            ..self
        }
    }
}

impl From<u64> for BigRat {
    fn from(i: u64) -> Self {
        Self {
            sign: Sign::Positive,
            num: i.into(),
            den: 1.into(),
        }
    }
}

impl From<BigUint> for BigRat {
    fn from(n: BigUint) -> Self {
        Self {
            sign: Sign::Positive,
            num: n,
            den: BigUint::from(1),
        }
    }
}

#[derive(Debug)]
enum FormattedBigRatType {
    // optional int, followed by a string (empty, "i" or " i"), followed by
    // whether to wrap the number in parentheses
    Integer(Option<FormattedBigUint>, &'static str, bool),
    // optional int (for mixed fractions)
    // optional int (numerator)
    // string (empty, "i" or " i")
    // '/'
    // int (denominator)
    // string (empty or " i") (used for mixed fractions, e.g. 1 2/3 i)
    // bool (whether or not to wrap the fraction in parentheses)
    Fraction(
        Option<FormattedBigUint>,
        Option<FormattedBigUint>,
        &'static str,
        FormattedBigUint,
        &'static str,
        bool,
    ),
    // string representation of decimal number (may or may not contain recurring digits)
    Decimal(String),
}

#[must_use]
#[derive(Debug)]
pub struct FormattedBigRat {
    // whether or not to print a minus sign
    sign: Sign,
    ty: FormattedBigRatType,
}

impl fmt::Display for FormattedBigRat {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.sign == Sign::Negative {
            write!(f, "-")?;
        }
        match &self.ty {
            FormattedBigRatType::Integer(int, isuf, use_parens) => {
                if *use_parens {
                    write!(f, "(")?;
                }
                if let Some(int) = int {
                    write!(f, "{}", int)?;
                }
                write!(f, "{}", isuf)?;
                if *use_parens {
                    write!(f, ")")?;
                }
            }
            FormattedBigRatType::Fraction(integer, num, isuf, den, isuf2, use_parens) => {
                if *use_parens {
                    write!(f, "(")?;
                }
                if let Some(integer) = integer {
                    write!(f, "{} ", integer)?;
                }
                if let Some(num) = num {
                    write!(f, "{}", num)?;
                }
                write!(f, "{}/{}{}", isuf, den, isuf2)?;
                if *use_parens {
                    write!(f, ")")?;
                }
            }
            FormattedBigRatType::Decimal(s) => write!(f, "{}", s)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::sign::Sign;
    use super::BigRat;
    use crate::err::{IntErr, Never};
    use crate::num::biguint::BigUint;

    #[test]
    fn test_bigrat_from() {
        BigRat::from(2);
        BigRat::from(0);
        BigRat::from(u64::MAX);
        BigRat::from(u64::from(u32::MAX));
    }

    #[test]
    fn test_addition() -> Result<(), IntErr<Never, crate::interrupt::Never>> {
        let int = &crate::interrupt::Never::default();
        let two = BigRat::from(2);
        assert_eq!(two.clone().add(two, int)?, BigRat::from(4));
        Ok(())
    }

    #[test]
    fn test_cmp() {
        assert!(
            BigRat {
                sign: Sign::Positive,
                num: BigUint::from(16),
                den: BigUint::from(9)
            } < BigRat::from(2)
        )
    }

    #[test]
    fn test_cmp_2() {
        assert!(
            BigRat {
                sign: Sign::Positive,
                num: BigUint::from(36),
                den: BigUint::from(49)
            } < BigRat {
                sign: Sign::Positive,
                num: BigUint::from(3),
                den: BigUint::from(4)
            }
        )
    }
}
