use core::fmt::{self, Debug, Formatter};

use crate::combinators::describe_grouped;
use crate::{Expr, Precedence};

/// Anything with a length: strings, slices, arrays and (with `alloc`) the standard collections.
pub trait Length {
    fn length(&self) -> usize;
}

/// Anything whose items can be iterated by reference.
pub trait Items {
    type Item;

    fn items(&self) -> impl Iterator<Item = &Self::Item>;
}

impl<L: Length + ?Sized> Length for &L {
    fn length(&self) -> usize {
        (**self).length()
    }
}

impl<C: Items + ?Sized> Items for &C {
    type Item = C::Item;

    fn items(&self) -> impl Iterator<Item = &Self::Item> {
        (**self).items()
    }
}

impl Length for str {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> Length for [T] {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> Items for [T] {
    type Item = T;

    fn items(&self) -> impl Iterator<Item = &T> {
        self.iter()
    }
}

impl<T, const N: usize> Length for [T; N] {
    fn length(&self) -> usize {
        N
    }
}

impl<T, const N: usize> Items for [T; N] {
    type Item = T;

    fn items(&self) -> impl Iterator<Item = &T> {
        self.iter()
    }
}

#[cfg(feature = "alloc")]
mod alloc_impls {
    use alloc::collections::{BTreeMap, BTreeSet, LinkedList, VecDeque};
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::{Items, Length};

    impl Length for String {
        fn length(&self) -> usize {
            self.len()
        }
    }

    impl<K, V> Length for BTreeMap<K, V> {
        fn length(&self) -> usize {
            self.len()
        }
    }

    macro_rules! collection {
        ($($ty:ident),*) => {
            $(
                impl<T> Length for $ty<T> {
                    fn length(&self) -> usize {
                        self.len()
                    }
                }

                impl<T> Items for $ty<T> {
                    type Item = T;

                    fn items(&self) -> impl Iterator<Item = &T> {
                        self.iter()
                    }
                }
            )*
        };
    }

    collection!(Vec, VecDeque, BTreeSet, LinkedList);
}

/// Has a length of zero.
pub struct IsEmpty;

impl<T: Length + ?Sized> Expr<T> for IsEmpty {
    fn check(&self, actual: &T) -> bool {
        actual.length() == 0
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Expected empty, got length {}", actual.length())
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("empty")
    }
}

/// A length matching `expr`.
pub struct HasLength<E> {
    pub expr: E,
}

impl<T: Length + ?Sized, E: Expr<usize>> Expr<T> for HasLength<E> {
    fn check(&self, actual: &T) -> bool {
        self.expr.check(&actual.length())
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Expected ")?;
        <Self as Expr<T>>::describe(self, f)?;
        write!(f, ", got {}", actual.length())
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("length ")?;
        describe_grouped(&self.expr, Precedence::Atom, f)
    }
}

/// Some item equals `value`.
pub struct Contains<U> {
    pub value: U,
}

impl<C, U> Expr<C> for Contains<U>
where
    C: Items + ?Sized,
    C::Item: PartialEq<U>,
    U: Debug,
{
    fn check(&self, actual: &C) -> bool {
        actual.items().any(|item| *item == self.value)
    }

    fn explain(&self, _actual: &C, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Expected collection containing {:?}, but it did not",
            self.value
        )
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "containing {:?}", self.value)
    }
}

/// At least one item matches `expr`.
pub struct AnyItem<E> {
    pub expr: E,
}

impl<C, E> Expr<C> for AnyItem<E>
where
    C: Items + ?Sized,
    E: Expr<C::Item>,
{
    fn check(&self, actual: &C) -> bool {
        actual.items().any(|item| self.expr.check(item))
    }

    fn explain(&self, _actual: &C, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Expected ")?;
        <Self as Expr<C>>::describe(self, f)?;
        f.write_str(", but none was")
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("any item ")?;
        describe_grouped(&self.expr, Precedence::Atom, f)
    }
}

/// Every item matches `expr`. Vacuously true for an empty collection.
pub struct AllItems<E> {
    pub expr: E,
}

impl<C, E> Expr<C> for AllItems<E>
where
    C: Items + ?Sized,
    E: Expr<C::Item>,
{
    fn check(&self, actual: &C) -> bool {
        actual.items().all(|item| self.expr.check(item))
    }

    fn explain(&self, actual: &C, f: &mut Formatter<'_>) -> fmt::Result {
        match actual
            .items()
            .enumerate()
            .find(|(_, item)| !self.expr.check(item))
        {
            Some((i, item)) => {
                write!(f, "Item at index {i}: ")?;
                self.expr.explain(item, f)
            }
            None => {
                f.write_str("Expected ")?;
                <Self as Expr<C>>::describe(self, f)
            }
        }
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("all items ")?;
        describe_grouped(&self.expr, Precedence::Atom, f)
    }
}

/// Equal to one of `values`.
pub struct OneOf<V> {
    pub values: V,
}

impl<T, V> Expr<T> for OneOf<V>
where
    T: PartialEq<V::Item> + Debug + ?Sized,
    V: Items + Debug,
{
    fn check(&self, actual: &T) -> bool {
        self.values.items().any(|value| actual == value)
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Expected one of {:?}, got {:?}", self.values, actual)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "one of {:?}", self.values)
    }
}
