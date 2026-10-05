use core::fmt::{self, Debug, Formatter};
use core::ops::RangeBounds;

use crate::Expr;

macro_rules! comparison {
    ($(#[$doc:meta])* $name:ident, $label:literal, |$a:ident, $v:ident| $test:expr) => {
        $(#[$doc])*
        pub struct $name<T> {
            pub value: T,
        }

        impl<T: PartialOrd + Debug> Expr<T> for $name<T> {
            fn check(&self, actual: &T) -> bool {
                let ($a, $v) = (actual, &self.value);
                $test
            }

            fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, concat!("Expected ", $label, " {:?}, got {:?}"), self.value, actual)
            }

            fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, concat!($label, " {:?}"), self.value)
            }
        }
    };
}

comparison!(
    /// Strictly greater than `value`.
    GreaterThan, "greater than", |a, v| a > v
);
comparison!(
    /// Strictly less than `value`.
    LessThan, "less than", |a, v| a < v
);
comparison!(
    /// Greater than or equal to `value`.
    AtLeast, "at least", |a, v| a >= v
);
comparison!(
    /// Less than or equal to `value`.
    AtMost, "at most", |a, v| a <= v
);

/// Within `range`, e.g. `1..=5` or `..10`.
pub struct InRange<R> {
    pub range: R,
}

impl<T, R> Expr<T> for InRange<R>
where
    T: PartialOrd + Debug,
    R: RangeBounds<T> + Debug,
{
    fn check(&self, actual: &T) -> bool {
        self.range.contains(actual)
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Expected in range {:?}, got {:?}", self.range, actual)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "in range {:?}", self.range)
    }
}
