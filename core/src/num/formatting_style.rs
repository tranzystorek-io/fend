use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[must_use]
pub enum FormattingStyle {
    /// Print value as an improper fraction
    ExactFraction,
    /// Print as a mixed fraction, e.g. 1 1/2
    MixedFraction,
    /// Print as a float, possibly indicating recurring digits
    /// with parentheses, e.g. 7/9 => 0.(81)
    ExactFloat,
    /// Print with the given number of decimal places
    DecimalPlaces(usize),
    /// If exact and no recurring digits: ExactFloat, if complex/imag: MixedFraction,
    /// otherwise: DecimalPlaces(10)
    Auto,
    /// If not exact: DecimalPlaces(10). If no recurring digits: ExactFloat.
    /// Other numbers: MixedFraction, albeit possibly including fractions of pi
    Exact,
}

impl Default for FormattingStyle {
    fn default() -> Self {
        Self::Auto
    }
}

impl fmt::Display for FormattingStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::ExactFraction => write!(f, "fraction"),
            Self::MixedFraction => write!(f, "mixed_fraction"),
            Self::ExactFloat => write!(f, "float"),
            Self::Exact => write!(f, "exact"),
            Self::DecimalPlaces(d) => write!(f, "{} dp", d),
            Self::Auto => write!(f, "auto"),
        }
    }
}
