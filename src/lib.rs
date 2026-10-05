#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

mod adapters;
mod assert;
mod builder;
mod collections;
mod combinators;
mod compare;
mod display;
mod expr;
mod primitives;
mod strings;

pub use adapters::{Field, Named, Property, Satisfies};
pub use assert::Assert;
pub use builder::{Is, Negated, PendingAnd, PendingOr, Query, Start, Step};
pub use collections::{AllItems, AnyItem, Contains, HasLength, IsEmpty, Items, Length, OneOf};
pub use combinators::{And, Not, Or};
pub use compare::{AtLeast, AtMost, GreaterThan, InRange, LessThan};
pub use display::{Description, Explanation};
pub use expr::{Expr, ExprExt, Precedence};
pub use primitives::{AlwaysFalse, Equal, IsNone, NoAlternatives};
pub use strings::{ContainsStr, EndsWith, StartsWith};

/// The everyday imports: `use runit::prelude::*;`.
pub mod prelude {
    pub use crate::{Assert, Expr, ExprExt, Is};
}
