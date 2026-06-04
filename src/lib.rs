pub trait Constraint<T: ?Sized> {
    fn check(&self, actual: &T) -> ConstraintResult;

    fn describe(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
}

pub struct ConstraintResult {
    pub pass: bool,
    pub message: String,
}

impl ConstraintResult {
    pub fn pass() -> Self {
        Self {
            pass: true,
            message: String::new(),
        }
    }

    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            pass: false,
            message: message.into(),
        }
    }
}

#[macro_export]
macro_rules! assert_that {
    ($actual:expr, $constraint:expr $(,)?) => {{
        $crate::assert_that_ref(&$actual, $constraint);
    }};
}

pub fn assert_that_ref<T, C>(actual: &T, constraint: C)
where
    T: ?Sized,
    C: Constraint<T>,
{
    let result = constraint.check(actual);

    if !result.pass {
        panic!("{}", result.message);
    }
}

pub mod constraints {
    use crate::{Constraint, ConstraintResult};

    use std::fmt::Debug;

    pub struct EqualTo<T> {
        pub(crate) expected: T,
    }

    pub struct GreaterThan<T> {
        pub(crate) expected: T,
    }

    pub struct LessThan<T> {
        pub(crate) expected: T,
    }

    pub struct Contains<T> {
        pub(crate) needle: T,
    }

    impl<T, E> Constraint<T> for EqualTo<E>
    where
        T: PartialEq<E> + Debug,
        E: Debug,
    {
        fn check(&self, actual: &T) -> ConstraintResult {
            if *actual == self.expected {
                ConstraintResult::pass()
            } else {
                ConstraintResult::fail(format!("Expected {:?} but got {:?}", self.expected, actual))
            }
        }
    }

    impl<T, E> Constraint<T> for GreaterThan<E>
    where
        T: PartialOrd<E> + Debug,
        E: Debug,
    {
        fn check(&self, actual: &T) -> ConstraintResult {
            if *actual > self.expected {
                ConstraintResult::pass()
            } else {
                ConstraintResult::fail(format!(
                    "Expected greater than {:?} but got {:?}",
                    self.expected, actual
                ))
            }
        }
    }

    impl<T, E> Constraint<T> for LessThan<E>
    where
        T: PartialOrd<E> + Debug,
        E: Debug,
    {
        fn check(&self, actual: &T) -> ConstraintResult {
            if *actual < self.expected {
                ConstraintResult::pass()
            } else {
                ConstraintResult::fail(format!(
                    "Expected less than {:?} but got {:?}",
                    self.expected, actual
                ))
            }
        }
    }

    impl<T, S> Constraint<T> for Contains<S>
    where
        T: AsRef<str> + Debug,
        S: AsRef<str> + Debug,
    {
        fn check(&self, actual: &T) -> ConstraintResult {
            let actual_str = actual.as_ref();
            let needle = self.needle.as_ref();

            if actual_str.contains(needle) {
                ConstraintResult::pass()
            } else {
                ConstraintResult::fail(format!(
                    "expected {:?} to contain {:?}",
                    actual, self.needle
                ))
            }
        }
    }
}

pub mod combinators {
    use crate::{Constraint, ConstraintResult};

    pub struct And<A, B> {
        pub(crate) left: A,
        pub(crate) right: B,
    }

    pub struct Or<A, B> {
        pub(crate) left: A,
        pub(crate) right: B,
    }

    pub struct Not<A> {
        pub(crate) inner: A,
    }

    impl<T: ?Sized, A, B> crate::Constraint<T> for And<A, B>
    where
        A: Constraint<T>,
        B: Constraint<T>,
    {
        fn check(&self, actual: &T) -> ConstraintResult {
            let left_result = self.left.check(actual);
            if !left_result.pass {
                return left_result;
            }

            let right_result = self.right.check(actual);
            if !right_result.pass {
                return right_result;
            }

            ConstraintResult::pass()
        }
    }

    impl<T: ?Sized, A, B> Constraint<T> for Or<A, B>
    where
        A: Constraint<T>,
        B: Constraint<T>,
    {
        fn check(&self, actual: &T) -> ConstraintResult {
            let left_result = self.left.check(actual);
            if left_result.pass {
                return ConstraintResult::pass();
            }

            let right_result = self.right.check(actual);
            if right_result.pass {
                return ConstraintResult::pass();
            }

            ConstraintResult::fail(format!(
                "Both constraints failed:\nLeft: {}\nRight: {}",
                left_result.message, right_result.message
            ))
        }
    }

    impl<T: ?Sized, A> Constraint<T> for Not<A>
    where
        A: Constraint<T>,
    {
        fn check(&self, actual: &T) -> ConstraintResult {
            let inner_result = self.inner.check(actual);
            if inner_result.pass {
                ConstraintResult::fail(format!(
                    "Expected constraint not to pass, but it did. Constraint description: {}. Inner message: {}",
                    self.inner.describe(),
                    inner_result.message
                ))
            } else {
                ConstraintResult::pass()
            }
        }
    }
}

pub mod builder {
    use crate::{
        combinators::{And, Not, Or},
        constraints::{Contains, EqualTo, GreaterThan, LessThan},
    };

    pub trait Combine<R> {
        type Output;

        fn combine(self, right: R) -> Self::Output;
    }

    impl<L, R> Combine<R> for AndIsBuilder<L> {
        type Output = And<L, R>;

        fn combine(self, right: R) -> Self::Output {
            And {
                left: self.left,
                right,
            }
        }
    }

    impl<L, R> Combine<R> for OrIsBuilder<L> {
        type Output = Or<L, R>;

        fn combine(self, right: R) -> Self::Output {
            Or {
                left: self.left,
                right,
            }
        }
    }

    macro_rules! impl_constraint_methods_for {
        ($builder:ident<$left:ident>, $not_builder:ident<$left_not:ident>) => {
            impl<$left> $builder<$left> {
                pub fn equal_to<E>(self, expected: E) -> <Self as Combine<EqualTo<E>>>::Output
                where
                    Self: Combine<EqualTo<E>>,
                {
                    self.combine(EqualTo { expected })
                }

                pub fn greater_than<E>(
                    self,
                    expected: E,
                ) -> <Self as Combine<GreaterThan<E>>>::Output
                where
                    Self: Combine<GreaterThan<E>>,
                {
                    self.combine(GreaterThan { expected })
                }

                pub fn less_than<E>(self, expected: E) -> <Self as Combine<LessThan<E>>>::Output
                where
                    Self: Combine<LessThan<E>>,
                {
                    self.combine(LessThan { expected })
                }

                pub fn contains<S>(self, needle: S) -> <Self as Combine<Contains<S>>>::Output
                where
                    Self: Combine<Contains<S>>,
                {
                    self.combine(Contains { needle })
                }

                pub fn truthy(self) -> <Self as Combine<EqualTo<bool>>>::Output
                where
                    Self: Combine<EqualTo<bool>>,
                {
                    self.combine(EqualTo { expected: true })
                }

                pub fn falsey(self) -> <Self as Combine<EqualTo<bool>>>::Output
                where
                    Self: Combine<EqualTo<bool>>,
                {
                    self.combine(EqualTo { expected: false })
                }

                #[allow(clippy::should_implement_trait)]
                pub fn not(self) -> $not_builder<$left> {
                    $not_builder { left: self.left }
                }

                pub fn constraint<C>(self, constraint: C) -> <Self as Combine<C>>::Output
                where
                    Self: Combine<C>,
                {
                    self.combine(constraint)
                }
            }
        };
    }

    pub fn is() -> IsBuilder {
        IsBuilder
    }

    pub struct IsBuilder;

    pub struct AndBuilder<L> {
        pub(crate) left: L,
    }

    pub struct OrBuilder<L> {
        pub(crate) left: L,
    }

    pub struct AndIsBuilder<L> {
        pub(crate) left: L,
    }

    pub struct OrIsBuilder<L> {
        pub(crate) left: L,
    }

    pub struct NotBuilder;

    pub struct AndNotBuilder<L> {
        pub(crate) left: L,
    }

    pub struct OrNotBuilder<L> {
        pub(crate) left: L,
    }

    impl_constraint_methods_for!(AndIsBuilder<L>, AndNotBuilder<L>);
    impl_constraint_methods_for!(OrIsBuilder<L>, OrNotBuilder<L>);
    impl_constraint_methods_for!(AndBuilder<L>, AndNotBuilder<L>);
    impl_constraint_methods_for!(OrBuilder<L>, OrNotBuilder<L>);

    pub trait Chainable: Sized {
        fn and(self) -> AndBuilder<Self> {
            AndBuilder { left: self }
        }

        fn or(self) -> OrBuilder<Self> {
            OrBuilder { left: self }
        }
    }

    impl<C> Chainable for C {}

    impl IsBuilder {
        pub fn equal_to<T>(self, expected: T) -> EqualTo<T> {
            EqualTo { expected }
        }

        pub fn greater_than<T>(self, expected: T) -> GreaterThan<T> {
            GreaterThan { expected }
        }

        pub fn less_than<T>(self, expected: T) -> LessThan<T> {
            LessThan { expected }
        }

        pub fn contains<T>(self, needle: T) -> Contains<T> {
            Contains { needle }
        }

        pub fn truthy(self) -> EqualTo<bool> {
            EqualTo { expected: true }
        }

        pub fn falsey(self) -> EqualTo<bool> {
            EqualTo { expected: false }
        }

        #[allow(clippy::should_implement_trait)]
        pub fn not(self) -> NotBuilder {
            NotBuilder
        }

        pub fn constraint<C>(self, constraint: C) -> C {
            constraint
        }
    }

    impl NotBuilder {
        pub fn equal_to<T>(self, expected: T) -> Not<EqualTo<T>> {
            Not {
                inner: EqualTo { expected },
            }
        }

        pub fn greater_than<T>(self, expected: T) -> Not<GreaterThan<T>> {
            Not {
                inner: GreaterThan { expected },
            }
        }

        pub fn less_than<T>(self, expected: T) -> Not<LessThan<T>> {
            Not {
                inner: LessThan { expected },
            }
        }

        pub fn contains<T>(self, needle: T) -> Not<Contains<T>> {
            Not {
                inner: Contains { needle },
            }
        }

        pub fn truthy(self) -> Not<EqualTo<bool>> {
            Not {
                inner: EqualTo { expected: true },
            }
        }

        pub fn falsey(self) -> Not<EqualTo<bool>> {
            Not {
                inner: EqualTo { expected: false },
            }
        }

        pub fn constraint<C>(self, constraint: C) -> Not<C> {
            Not { inner: constraint }
        }
    }

    impl<L> AndNotBuilder<L> {
        pub fn equal_to<E>(self, expected: E) -> And<L, Not<EqualTo<E>>> {
            And {
                left: self.left,
                right: Not {
                    inner: EqualTo { expected },
                },
            }
        }

        pub fn greater_than<E>(self, expected: E) -> And<L, Not<GreaterThan<E>>> {
            And {
                left: self.left,
                right: Not {
                    inner: GreaterThan { expected },
                },
            }
        }

        pub fn less_than<E>(self, expected: E) -> And<L, Not<LessThan<E>>> {
            And {
                left: self.left,
                right: Not {
                    inner: LessThan { expected },
                },
            }
        }

        pub fn contains<E>(self, needle: E) -> And<L, Not<Contains<E>>> {
            And {
                left: self.left,
                right: Not {
                    inner: Contains { needle },
                },
            }
        }

        pub fn truthy(self) -> And<L, Not<EqualTo<bool>>> {
            And {
                left: self.left,
                right: Not {
                    inner: EqualTo { expected: true },
                },
            }
        }

        pub fn falsey(self) -> And<L, Not<EqualTo<bool>>> {
            And {
                left: self.left,
                right: Not {
                    inner: EqualTo { expected: false },
                },
            }
        }

        pub fn constraint<C>(self, constraint: C) -> And<L, Not<C>> {
            And {
                left: self.left,
                right: Not { inner: constraint },
            }
        }
    }

    impl<L> OrNotBuilder<L> {
        pub fn equal_to<E>(self, expected: E) -> Or<L, Not<EqualTo<E>>> {
            Or {
                left: self.left,
                right: Not {
                    inner: EqualTo { expected },
                },
            }
        }

        pub fn greater_than<E>(self, expected: E) -> Or<L, Not<GreaterThan<E>>> {
            Or {
                left: self.left,
                right: Not {
                    inner: GreaterThan { expected },
                },
            }
        }

        pub fn less_than<E>(self, expected: E) -> Or<L, Not<LessThan<E>>> {
            Or {
                left: self.left,
                right: Not {
                    inner: LessThan { expected },
                },
            }
        }

        pub fn contains<E>(self, needle: E) -> Or<L, Not<Contains<E>>> {
            Or {
                left: self.left,
                right: Not {
                    inner: Contains { needle },
                },
            }
        }

        pub fn truthy(self) -> Or<L, Not<EqualTo<bool>>> {
            Or {
                left: self.left,
                right: Not {
                    inner: EqualTo { expected: true },
                },
            }
        }

        pub fn falsey(self) -> Or<L, Not<EqualTo<bool>>> {
            Or {
                left: self.left,
                right: Not {
                    inner: EqualTo { expected: false },
                },
            }
        }

        pub fn constraint<C>(self, constraint: C) -> Or<L, Not<C>> {
            Or {
                left: self.left,
                right: Not { inner: constraint },
            }
        }
    }

    impl<L> AndBuilder<L> {
        pub fn is(self) -> AndIsBuilder<L> {
            AndIsBuilder { left: self.left }
        }
    }

    impl<L> OrBuilder<L> {
        pub fn is(self) -> OrIsBuilder<L> {
            OrIsBuilder { left: self.left }
        }
    }

    impl<L, R> Combine<R> for AndBuilder<L> {
        type Output = And<L, R>;

        fn combine(self, right: R) -> Self::Output {
            And {
                left: self.left,
                right,
            }
        }
    }

    impl<L, R> Combine<R> for OrBuilder<L> {
        type Output = Or<L, R>;

        fn combine(self, right: R) -> Self::Output {
            Or {
                left: self.left,
                right,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::{Chainable, is};

    #[test]
    fn test_number_constraints() {
        assert_that!(5, is().equal_to(5));
        assert_that!(5, is().equal_to(4).or().is().equal_to(5));
        assert_that!(
            5,
            is().equal_to(4)
                .or()
                .equal_to(5)
                .and()
                .is()
                .greater_than(3)
        );
        assert_that!(
            5,
            is().equal_to(5)
                .and()
                .is()
                .greater_than(3)
                .or()
                .less_than(0)
        );
        assert_that!(
            5,
            is().equal_to(5)
                .and()
                .is()
                .greater_than(3)
                .or()
                .less_than(0)
                .and()
                .is()
                .less_than(10)
        );
        assert_that!(5, is().not().equal_to(6));
        assert_that!(
            5,
            is().not()
                .constraint(is().equal_to(12039).or().is().equal_to(10382938))
                .and()
                .greater_than(3)
        );
    }

    #[test]
    fn test_string_constraints() {
        assert_that!("hello world", is().contains("world"));
        assert_that!(
            "hello world",
            is().contains("world").and().is().not().contains("goodbye")
        );
        assert_that!(
            "hello world",
            is().contains("world").or().is().contains("goodbye")
        );
        assert_that!(
            "hello world",
            is().contains("world")
                .or()
                .contains("goodbye")
                .and()
                .not()
                .contains("foo")
                .and()
                .constraint(is().constraint(is().constraint(is().constraint(
                    is().constraint(is().constraint(is().not().constraint(is().equal_to("lol"))))
                ))))
        )
    }
}
