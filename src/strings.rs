use core::fmt::{self, Formatter};

use crate::Expr;

macro_rules! string_expr {
    ($(#[$doc:meta])* $name:ident, $label:literal, $method:ident) => {
        $(#[$doc])*
        pub struct $name<S> {
            pub pattern: S,
        }

        impl<T: AsRef<str> + ?Sized, S: AsRef<str>> Expr<T> for $name<S> {
            fn check(&self, actual: &T) -> bool {
                actual.as_ref().$method(self.pattern.as_ref())
            }

            fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    concat!("Expected string ", $label, " {:?}, got {:?}"),
                    self.pattern.as_ref(),
                    actual.as_ref()
                )
            }

            fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, concat!($label, " {:?}"), self.pattern.as_ref())
            }
        }
    };
}

string_expr!(
    /// A string starting with `pattern`.
    StartsWith, "starting with", starts_with
);
string_expr!(
    /// A string ending with `pattern`.
    EndsWith, "ending with", ends_with
);
string_expr!(
    /// A string containing `pattern` as a substring.
    ContainsStr, "containing", contains
);
