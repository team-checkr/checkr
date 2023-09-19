use crate::ast::{AExpr, AOp, Array, BExpr, Function, Int, LogicOp, RelOp, Target, Variable};

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SemanticsError {
    #[error("division by zero")]
    DivisionByZero,
    #[error("negative exponent")]
    NegativeExponent,
    #[error("variable '{name}' not found")]
    VariableNotFound { name: String },
    #[error("array '{name}' not found")]
    ArrayNotFound { name: String },
    #[error("index {index} in '{name}' is out-of-bounds")]
    IndexOutOfBound { name: String, index: Int },
    #[error("no progression")]
    NoProgression,
    #[error("an arithmetic operation overflowed")]
    ArithmeticOverflow,
    #[error("tried to evaluate a quantified expression")]
    EvaluateQuantifier,
    #[error("tried to evaluate function where argument was outside of domain")]
    OutsideFunctionDomain,
}

pub trait SemanticsContext {
    fn variable(&self, var: &Variable) -> Result<Int, SemanticsError>;
    fn array_element(&self, array: &Array, index: Int) -> Result<Int, SemanticsError>;
    fn array_length(&self, array: &Array) -> Result<Int, SemanticsError>;
    fn array_count(&self, array: &Array, element: Int) -> Result<Int, SemanticsError>;
}

pub struct EmptySemanticsContext;

impl SemanticsContext for EmptySemanticsContext {
    fn variable(&self, var: &Variable) -> Result<Int, SemanticsError> {
        Err(SemanticsError::VariableNotFound {
            name: var.to_string(),
        })
    }

    fn array_element(&self, array: &Array, _index: Int) -> Result<Int, SemanticsError> {
        Err(SemanticsError::ArrayNotFound {
            name: array.to_string(),
        })
    }

    fn array_length(&self, array: &Array) -> Result<Int, SemanticsError> {
        Err(SemanticsError::ArrayNotFound {
            name: array.to_string(),
        })
    }

    fn array_count(&self, array: &Array, _element: Int) -> Result<Int, SemanticsError> {
        Err(SemanticsError::ArrayNotFound {
            name: array.to_string(),
        })
    }
}

impl AExpr {
    pub fn semantics<S: SemanticsContext>(&self, cx: &S) -> Result<Int, SemanticsError> {
        Ok(match self {
            AExpr::Number(n) => *n,
            AExpr::Reference(Target::Variable(x)) => cx.variable(x)?,
            AExpr::Reference(Target::Array(arr, idx)) => {
                let idx = idx.semantics(cx)?;
                cx.array_element(arr, idx)?
            }
            AExpr::Binary(l, op, r) => op.semantic(l.semantics(cx)?, r.semantics(cx)?)?,
            AExpr::Minus(n) => n
                .semantics(cx)?
                .checked_neg()
                .ok_or(SemanticsError::ArithmeticOverflow)?,
            AExpr::Function(f) => match f {
                Function::Division(l, r) => {
                    AOp::Divide.semantic(l.semantics(cx)?, r.semantics(cx)?)?
                }
                Function::Min(x, y) => x.semantics(cx)?.min(y.semantics(cx)?),
                Function::Max(x, y) => x.semantics(cx)?.max(y.semantics(cx)?),
                Function::Count(arr, x) | Function::LogicalCount(arr, x) => {
                    let x = x.semantics(cx)?;
                    cx.array_count(arr, x)?
                }
                Function::Length(arr) | Function::LogicalLength(arr) => cx.array_length(arr)?,
                Function::Fac(x) => {
                    let x = x.semantics(cx)?;
                    if x < 0 {
                        return Err(SemanticsError::OutsideFunctionDomain);
                    }
                    (1..=x)
                        .try_fold(1 as Int, |acc, x| acc.checked_mul(x))
                        .ok_or(SemanticsError::ArithmeticOverflow)?
                }
                Function::Fib(x) => {
                    let x = x.semantics(cx)?;
                    if x < 0 {
                        return Err(SemanticsError::OutsideFunctionDomain);
                    }
                    (0..x)
                        .try_fold((0 as Int, 1), |(a, b), _| Some((b, a.checked_add(b)?)))
                        .map(|(x, _)| x)
                        .ok_or(SemanticsError::ArithmeticOverflow)?
                }
            },
        })
    }
}

impl AOp {
    pub fn semantic(&self, l: Int, r: Int) -> Result<Int, SemanticsError> {
        Ok(match self {
            AOp::Plus => l.checked_add(r).ok_or(SemanticsError::ArithmeticOverflow)?,
            AOp::Minus => l.checked_sub(r).ok_or(SemanticsError::ArithmeticOverflow)?,
            AOp::Times => l.checked_mul(r).ok_or(SemanticsError::ArithmeticOverflow)?,
            AOp::Divide => {
                if r != 0 {
                    l / r
                } else {
                    return Err(SemanticsError::DivisionByZero);
                }
            }
            AOp::Pow => {
                if r >= 0 {
                    l.checked_pow(r as _)
                        .ok_or(SemanticsError::ArithmeticOverflow)?
                } else {
                    return Err(SemanticsError::NegativeExponent);
                }
            }
        })
    }
}

impl BExpr {
    pub fn semantics<S: SemanticsContext>(&self, cx: &S) -> Result<bool, SemanticsError> {
        Ok(match self {
            BExpr::Bool(b) => *b,
            BExpr::Rel(l, op, r) => op.semantic(l.semantics(cx)?, r.semantics(cx)?),
            BExpr::Logic(l, op, r) => op.semantic(l.semantics(cx)?, || r.semantics(cx))?,
            BExpr::Not(b) => !b.semantics(cx)?,
            BExpr::Quantified(_, _, _) => return Err(SemanticsError::EvaluateQuantifier),
        })
    }
}

impl RelOp {
    pub fn semantic(&self, l: Int, r: Int) -> bool {
        match self {
            RelOp::Eq => l == r,
            RelOp::Ne => l != r,
            RelOp::Gt => l > r,
            RelOp::Ge => l >= r,
            RelOp::Lt => l < r,
            RelOp::Le => l <= r,
        }
    }
}

impl LogicOp {
    pub fn semantic(
        &self,
        l: bool,
        r: impl FnOnce() -> Result<bool, SemanticsError>,
    ) -> Result<bool, SemanticsError> {
        Ok(match self {
            LogicOp::And => l && r()?,
            LogicOp::Land => {
                let r = r()?;
                l && r
            }
            LogicOp::Or => l || r()?,
            LogicOp::Lor => {
                let r = r()?;
                l || r
            }
            LogicOp::Implies => {
                let r = r()?;
                !l || r
            }
        })
    }
}
