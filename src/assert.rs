use crate::{Expr, ExprExt};

pub struct Assert;

impl Assert {
    #[track_caller]
    pub fn that<T, C>(actual: &T, constraint: C)
    where
        T: ?Sized,
        C: Expr<T>,
    {
        if let Err(why) = constraint.validate(actual) {
            panic!("{why}");
        }
    }
}
