use crate::Uint;

/// Describes a number that can be expressed as the quotient of two integers.
///
/// Note that here we only concern the fraction's absolute value. Both the
/// numerator and denominator here are negative. This trait is intended to be
/// used together with [`Sign`] To account for negative fractions.
pub trait Fraction<U> {
    fn numerator(&self) -> Uint<U>;

    fn denominator() -> Uint<U>;
}
