#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

mod assert;
mod builder;
mod combinators;
mod display;
mod expr;
mod primitives;

pub use assert::Assert;
pub use builder::{Is, Negated, PendingAnd, PendingOr, Query, Start, Step};
pub use combinators::{And, Not, Or};
pub use display::{Description, Explanation};
pub use expr::{Expr, Precedence};
pub use primitives::{AlwaysFalse, Equal, IsNone, NoAlternatives};
