use std::ops::Mul;

use p3_air::{PairCol, VirtualPairCol};
use p3_field::{AbstractField, Field};

/// An expression that represents an abstract linear combination of columns.
///
/// The `apply` method is used to evaluate the expression given the preprocessed and main trace
/// rows. For example, given a `VirtualColumn` which represents the linear combination
/// `a * Main[0] + b * Preprocessed[1] + c`, the `apply` method will return the value of `a *
/// main[0] + b * preprocessed[1] + c`.
pub trait VirtualColumn<F: Field>: Sync {
    /// Evaluate the expression given the preprocessed and main trace rows.
    fn apply<Expr, Var>(&self, preprocessed: &[Var], main: &[Var]) -> Expr
    where
        F: Into<Expr>,
        Expr: AbstractField + Mul<F, Output = Expr>,
        Var: Into<Expr> + Copy;
}

/// A constant virtual column.
#[derive(Debug, Clone)]
pub struct VirtualPairColView<'a, F: Field> {
    column_weights: &'a [(PairCol, F)],
    constant: F,
}

impl<F: Field> VirtualColumn<F> for VirtualPairCol<F> {
    fn apply<Expr, Var>(&self, preprocessed: &[Var], main: &[Var]) -> Expr
    where
        F: Into<Expr>,
        Expr: AbstractField + Mul<F, Output = Expr>,
        Var: Into<Expr> + Copy,
    {
        self.apply(preprocessed, main)
    }
}

impl<'a, F: Field> VirtualColumn<F> for VirtualPairColView<'a, F> {
    fn apply<Expr, Var>(&self, preprocessed: &[Var], main: &[Var]) -> Expr
    where
        F: Into<Expr>,
        Expr: AbstractField + Mul<F, Output = Expr>,
        Var: Into<Expr> + Copy,
    {
        let mut result = self.constant.into();
        for (column, weight) in self.column_weights {
            result += column.get(preprocessed, main).into() * *weight;
        }
        result
    }
}

impl<'a, F: Field> From<&'a VirtualPairCol<F>> for VirtualPairColView<'a, F> {
    #[inline]
    fn from(v: &'a VirtualPairCol<F>) -> Self {
        Self {
            column_weights: &v.column_weights,
            constant: v.constant,
        }
    }
}
