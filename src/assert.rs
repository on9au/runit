use crate::{Explanation, Expr};

pub struct Assert;

impl Assert {
    #[track_caller]
    pub fn that<T, C>(actual: &T, constraint: C)
    where
        T: ?Sized,
        C: Expr<T>,
    {
        if !constraint.check(actual) {
            panic!("{}", Explanation::new(&constraint, actual));
        }
    }
}
