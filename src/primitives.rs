use core::fmt::{self, Debug, Formatter};

use crate::Expr;

pub struct AlwaysFalse;
/// The empty set of alternatives a builder chain starts from. Contributes nothing to
/// failure messages or descriptions.
pub struct NoAlternatives;
pub struct IsNone;
pub struct Equal<T> {
    pub value: T,
}

impl<T: ?Sized> Expr<T> for AlwaysFalse {
    fn check(&self, _actual: &T) -> bool {
        false
    }

    fn explain(&self, _actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Condition will always fail.")
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("always false")
    }
}

impl<T: ?Sized> Expr<T> for NoAlternatives {
    fn check(&self, _actual: &T) -> bool {
        false
    }

    fn explain(&self, _actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("No alternatives to match.")
    }

    fn describe(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }

    fn alternatives(&self) -> usize {
        0
    }

    fn explain_alternatives(&self, _actual: &T, _f: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl<T> Expr<Option<T>> for IsNone {
    fn check(&self, actual: &Option<T>) -> bool {
        actual.is_none()
    }

    fn explain(&self, _actual: &Option<T>, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Expected None, got Some(_)")
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("None")
    }
}

impl<T: PartialEq + Debug> Expr<T> for Equal<T> {
    fn check(&self, actual: &T) -> bool {
        &self.value == actual
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Expected {:?}, got {:?}", self.value, actual)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "equal to {:?}", self.value)
    }
}
