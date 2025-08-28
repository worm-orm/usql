use super::{BinaryExpression, BinaryOperator, Expression};

macro_rules! op_impl {
    ($method:ident, $op: ident) => {
        fn $method<'a, E: Expression<'a>>(self, e: E) -> BinaryExpression<Self, E>
        where
            Self: 'a,
        {
            BinaryExpression::new(self, e, BinaryOperator::$op)
        }
    };
}

pub trait ExpressionExt<'val>: Expression<'val> + Sized {
    // Operators
    op_impl!(neq, NotEq);
    op_impl!(eql, Eq);
    op_impl!(lt, Lt);
    op_impl!(lte, Lte);
    op_impl!(gt, Gt);
    op_impl!(gte, Gte);
    op_impl!(like, Like);
    op_impl!(has, In);
    op_impl!(matching, Match);
}

impl<'val, T> ExpressionExt<'val> for T where T: Expression<'val> {}
