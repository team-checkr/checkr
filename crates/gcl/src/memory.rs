use std::collections::BTreeMap;

use itertools::chain;
use serde::{Deserialize, Serialize};

use crate::ast::{Array, Target, Variable};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Memory<T, A = T> {
    pub variables: BTreeMap<Variable, T>,
    pub arrays: BTreeMap<Array, A>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryRef<'a, T, A> {
    Variable(&'a Variable, &'a T),
    Array(&'a Array, &'a A),
}

impl<T, A> Memory<T, A> {
    pub fn from_targets(
        targets: impl IntoIterator<Item = Target>,
        mut f_var: impl FnMut(&Variable) -> T,
        mut f_array: impl FnMut(&Array) -> A,
    ) -> Self {
        let mut variables = BTreeMap::new();
        let mut arrays = BTreeMap::new();

        for t in targets {
            match t {
                Target::Variable(var) => {
                    let value = f_var(&var);
                    variables.insert(var, value);
                }
                Target::Array(arr, ()) => {
                    let value = f_array(&arr);
                    arrays.insert(arr, value);
                }
            }
        }

        Self { variables, arrays }
    }
    pub fn from_targets_with<W>(
        targets: impl IntoIterator<Item = Target>,
        mut with: W,
        mut f_var: impl for<'a, 'b> FnMut(&'a mut W, &'b Variable) -> T,
        mut f_array: impl for<'a, 'b> FnMut(&'a mut W, &'b Array) -> A,
    ) -> Self {
        let mut variables = BTreeMap::new();
        let mut arrays = BTreeMap::new();

        for t in targets {
            match t {
                Target::Variable(var) => {
                    let value = f_var(&mut with, &var);
                    variables.insert(var, value);
                }
                Target::Array(arr, ()) => {
                    let value = f_array(&mut with, &arr);
                    arrays.insert(arr, value);
                }
            }
        }

        Self { variables, arrays }
    }

    pub fn iter(&self) -> impl Iterator<Item = MemoryRef<'_, T, A>> + Clone {
        chain!(
            self.variables
                .iter()
                .map(|(var, value)| MemoryRef::Variable(var, value)),
            self.arrays
                .iter()
                .map(|(arr, value)| MemoryRef::Array(arr, value)),
        )
    }

    pub fn with_var(mut self, var: &Variable, value: T) -> Self {
        *self
            .variables
            .get_mut(var)
            .unwrap_or_else(|| panic!("variable `{var}` not declared")) = value;
        self
    }
    pub fn get_var(&self, var: &Variable) -> Option<&T> {
        self.variables.get(var)
    }
    pub fn get_arr(&self, arr: &Array) -> Option<&A> {
        self.arrays.get(arr)
    }
}

impl<T, A> MemoryRef<'_, T, A> {
    pub fn target(&self) -> Target {
        match self {
            MemoryRef::Variable(t, _) => Target::Variable((*t).clone()),
            MemoryRef::Array(t, _) => Target::Array((*t).clone(), ()),
        }
    }
}

impl<T> MemoryRef<'_, T, T> {
    pub fn value(&self) -> &T {
        match self {
            MemoryRef::Variable(_, v) | MemoryRef::Array(_, v) => v,
        }
    }
}

impl<T, A> std::fmt::Display for MemoryRef<'_, T, A>
where
    T: std::fmt::Display,
    A: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryRef::Variable(a, b) => write!(f, "{a} = {b}"),
            MemoryRef::Array(a, b) => write!(f, "{a} = {b}"),
        }
    }
}
