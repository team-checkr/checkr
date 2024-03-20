use crate::{
    ast::{AExpr, AOp, Array, BExpr, Int, LogicOp, RelOp, Target, Variable},
    pg::Action,
};

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

pub trait SemanticsContext: Sized + Clone {
    fn variable(&self, var: &Variable) -> Result<Int, SemanticsError>;
    fn array_element(&self, array: &Array, index: Int) -> Result<Int, SemanticsError>;
    fn set_variable(&self, var: &Variable, value: Int) -> Result<Self, SemanticsError>;
    fn set_array_element(
        &self,
        array: &Array,
        index: Int,
        value: Int,
    ) -> Result<Self, SemanticsError>;
    fn array_length(&self, array: &Array) -> Result<Int, SemanticsError>;
    fn array_count(&self, array: &Array, element: Int) -> Result<Int, SemanticsError>;
}

#[derive(Clone)]
pub struct EmptySemanticsContext;

impl SemanticsContext for EmptySemanticsContext {
    fn variable(&self, var: &Variable) -> Result<Int, SemanticsError> {
        Err(SemanticsError::VariableNotFound {
            name: var.to_string(),
        })
    }

    fn set_variable(&self, _var: &Variable, _value: Int) -> Result<Self, SemanticsError> {
        Ok(self.clone())
    }

    fn array_element(&self, array: &Array, _index: Int) -> Result<Int, SemanticsError> {
        Err(SemanticsError::ArrayNotFound {
            name: array.to_string(),
        })
    }

    fn set_array_element(
        &self,
        _array: &Array,
        _index: Int,
        _value: Int,
    ) -> Result<Self, SemanticsError> {
        Ok(self.clone())
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
        })
    }
}

impl Action {
    pub fn semantics<S: SemanticsContext>(&self, cx: &S) -> Result<S, SemanticsError> {
        match self {
            Action::Assignment(Target::Variable(x), a) => {
                let value = a.semantics(cx)?;
                cx.set_variable(x, value)
            }
            Action::Assignment(Target::Array(arr, idx), a) => {
                let idx = idx.semantics(cx)?;
                let value = a.semantics(cx)?;
                cx.set_array_element(arr, idx, value)
            }
            Action::Skip => Ok(cx.clone()),
            Action::Condition(b) => {
                if b.semantics(cx)? {
                    Ok(cx.clone())
                } else {
                    Err(SemanticsError::NoProgression)
                }
            }
        }
    }
}
