use std::fmt;

use indexmap::IndexMap;
use itertools::{Either, Itertools};
use mcltl::ltl::expression::Literal;

use crate::{
    ast::{
        AExpr, AOp, BExpr, Command, CommandKind, Commands, Field, Function, Int, LTLFormula,
        Locator, LogicOp, Operation, RelOp, Target, TupleSpace, TupleSpaceSize, TupleSpaceType,
        Variable,
    },
    ast_ext::FreeVariables,
    parse::SourceSpan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InstrPtr(u32);
impl InstrPtr {
    fn bump(&self) -> InstrPtr {
        InstrPtr(self.0 + 1)
    }
}

#[derive(Debug)]
enum Instr {
    Nop,
    Assign(u32, AExpr),
    Branch {
        choices: Vec<(BExpr, InstrPtr)>,
        otherwise: Option<InstrPtr>,
    },
    Goto(InstrPtr),
    Halt,
    Put(TupleSpaceSize, u32, Vec<AExpr>),
    Get(TupleSpaceType, u32, Vec<Field>),
    Query(TupleSpaceType, u32, Vec<Field>),
}

#[derive(Debug)]
pub struct Program {
    variables: Vec<Variable>,
    tuple_spaces: Vec<TupleSpaceMeta>,
    instrs: Vec<Instr>,
    entry_points: Vec<InstrPtr>,
    source_map: Vec<Option<SourceSpan>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleSpaceMeta {
    pub name: Variable,
    pub space_type: TupleSpaceType,
    pub size: TupleSpaceSize,
}

impl std::ops::Index<InstrPtr> for Program {
    type Output = Instr;

    fn index(&self, ptr: InstrPtr) -> &Instr {
        &self.instrs[ptr.0 as usize]
    }
}

impl Program {
    pub fn compile(
        cmdss: &[Commands<(), ()>],
        additional_vars: impl IntoIterator<Item = Variable>,
        tuple_spaces: IndexMap<Variable, TupleSpace>,
    ) -> Program {
        let mut p = Program {
            variables: cmdss
                .iter()
                .flat_map(|cmds| {
                    cmds.fv().into_iter().filter_map(|t| match t {
                        Target::Variable(var) => Some(var),
                        Target::Array(_, _) => None,
                    })
                })
                .chain(additional_vars)
                .sorted()
                .dedup()
                .collect(),
            instrs: Vec::new(),
            entry_points: Vec::new(),
            source_map: Vec::new(),
            tuple_spaces: tuple_spaces
                .into_iter()
                .map(|(var, ts)| TupleSpaceMeta {
                    name: var,
                    space_type: ts.space_type,
                    size: ts.size,
                })
                .collect(),
        };

        for cmds in cmdss {
            let entry = p.current();
            p.entry_points.push(entry);
            p.compile_commands(cmds);
            p.push(
                Instr::Halt,
                cmds.0.last().map(|cmd| cmd.span.cursor_at_end()),
            );
        }

        p
    }

    pub fn initial_state(
        &self,
        memory: impl Fn(&Variable) -> i32,
        tuple_space_memory: Vec<Vec<Vec<Int>>>,
    ) -> State {
        State {
            ptrs: self.entry_points.clone(),
            memory: self.variables.iter().map(memory).collect(),
            tuple_spaces: tuple_space_memory,
        }
    }

    pub fn variables(&self) -> impl Iterator<Item = &'_ Variable> {
        self.variables.iter()
    }

    fn variable_index(&self, name: &str) -> Option<u32> {
        self.variables
            .iter()
            .position(|v| v.0 == name)
            .map(|idx| idx as _)
    }

    fn tuple_space_index(&self, name: &str) -> Option<u32> {
        self.tuple_spaces
            .iter()
            .position(|v| v.name.0 == name)
            .map(|idx| idx as _)
    }

    fn current(&self) -> InstrPtr {
        InstrPtr(self.instrs.len() as _)
    }

    fn push(&mut self, instr: Instr, src: Option<SourceSpan>) -> InstrPtr {
        let ptr = self.current();
        self.instrs.push(instr);
        self.source_map.push(src);
        ptr
    }

    fn set(&mut self, ptr: InstrPtr, instr: Instr) {
        self.instrs[ptr.0 as usize] = instr;
    }

    fn compile_commands(&mut self, cmds: &Commands<(), ()>) {
        for cmd in &cmds.0 {
            self.compile_command(cmd);
        }
    }

    fn compile_command(&mut self, cmd: &Command<(), ()>) {
        match &cmd.kind {
            CommandKind::Assignment(t, e) => {
                let index = self.variable_index(t.name()).unwrap();
                self.push(Instr::Assign(index, e.clone()), Some(cmd.span));
            }
            CommandKind::Skip => {
                self.push(Instr::Nop, Some(cmd.span));
            }
            CommandKind::Placeholder => {
                self.push(Instr::Nop, Some(cmd.span));
            }
            CommandKind::If(guards) => {
                let head = self.push(Instr::Nop, Some(cmd.span));
                let mut choices = Vec::new();
                let mut exits = Vec::new();
                for guard in guards {
                    choices.push((guard.guard.clone(), self.current()));
                    self.compile_commands(&guard.cmds);
                    exits.push(self.current());
                    self.push(Instr::Nop, Some(cmd.span));
                }
                self.set(
                    head,
                    Instr::Branch {
                        choices,
                        otherwise: None,
                    },
                );
                for exit in exits {
                    self.set(exit, Instr::Goto(self.current()));
                }
            }
            CommandKind::Loop(_, guards) => {
                let head = self.push(Instr::Nop, Some(cmd.span));
                let mut choices = Vec::new();
                for guard in guards {
                    choices.push((guard.guard.clone(), self.current()));
                    self.compile_commands(&guard.cmds);
                    self.push(Instr::Goto(head), Some(cmd.span));
                }
                self.set(
                    head,
                    Instr::Branch {
                        choices,
                        otherwise: Some(self.current()),
                    },
                );
            }
            CommandKind::O(Operation::Put(target, args)) => {
                let index = self.tuple_space_index(target.name()).unwrap();
                let tuple_max_size = self.tuple_spaces[index as usize].size.clone();
                self.push(
                    Instr::Put(tuple_max_size, index, args.clone()),
                    Some(cmd.span),
                );
            }
            CommandKind::O(Operation::Get(target, args)) => {
                let index = self.tuple_space_index(target.name()).unwrap();
                let tuple_type = self.tuple_spaces[index as usize].space_type.clone();
                self.push(Instr::Get(tuple_type, index, args.clone()), Some(cmd.span));
            }
            CommandKind::O(Operation::Query(target, args)) => {
                let index = self.tuple_space_index(target.name()).unwrap();
                let tuple_type = self.tuple_spaces[index as usize].space_type.clone();
                self.push(
                    Instr::Query(tuple_type, index, args.clone()),
                    Some(cmd.span),
                );
            }
        }
    }
}

type Memory = Vec<i32>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    ptrs: Vec<InstrPtr>,
    memory: Memory,
    tuple_spaces: Vec<Vec<Vec<Int>>>,
}

#[derive(Debug)]
pub enum StepError {
    DivisionByZero,
    NegativeFactorial,
    NegativeFibonacci,
    NegativePower,
    Stuck,
    Halt,
    HitOld,
}

impl State {
    pub fn step<'a>(&'a self, p: &'a Program) -> impl Iterator<Item = State> + 'a {
        self.step_inner(p).flatten()
    }
    fn step_inner<'a>(
        &'a self,
        p: &'a Program,
    ) -> impl Iterator<Item = Result<State, StepError>> + 'a {
        self.ptrs
            .iter()
            .enumerate()
            .flat_map(|(i, _)| match self.step_exe(p, i) {
                Ok(inner) => Either::Left(inner.map(Ok)),
                Err(err) => Either::Right([Err(err)].into_iter()),
            })
            .map(|s| Ok(s?.follow_gotos(p)))
    }
    fn follow_gotos(mut self, p: &Program) -> State {
        for ptr in self.ptrs.iter_mut() {
            loop {
                match &p[*ptr] {
                    Instr::Goto(q) => *ptr = *q,
                    Instr::Nop => *ptr = ptr.bump(),
                    _ => break,
                }
            }
        }
        self
    }
    pub fn spans<'a>(&'a self, p: &'a Program) -> impl Iterator<Item = SourceSpan> + 'a {
        self.ptrs
            .iter()
            .filter_map(|ptr| p.source_map[ptr.0 as usize])
    }
    pub fn variables<'a>(&'a self, p: &'a Program) -> impl Iterator<Item = (&'a Variable, i32)> {
        p.variables().zip(self.memory.iter().copied())
    }
    fn step_exe<'a>(
        &'a self,
        p: &'a Program,
        execution: usize,
    ) -> Result<impl Iterator<Item = State> + 'a, StepError> {
        Ok(self
            .step_at(p, self.ptrs[execution])?
            .map(move |(mem, tuple_spaces, ptr)| {
                let mut ptrs = self.ptrs.clone();
                ptrs[execution] = ptr;
                State {
                    ptrs,
                    memory: mem,
                    tuple_spaces,
                }
            }))
    }
    pub fn is_terminated(&self, p: &Program) -> bool {
        self.step_inner(p)
            .all(|t| matches!(t, Err(StepError::Halt)))
    }
    pub fn is_stuck(&self, p: &Program) -> bool {
        let all_stuck_or_halted = self
            .step_inner(p)
            .all(|t| matches!(t, Err(StepError::Stuck | StepError::Halt)));
        let any_stuck = self
            .step_inner(p)
            .any(|t| matches!(t, Err(StepError::Stuck)));

        all_stuck_or_halted && any_stuck
    }

    fn matches(&self, t: &Vec<i32>, fields: &Vec<Field>, p: &Program) -> Result<bool, StepError> {
        if t.len() != fields.len() {
            return Ok(false);
        }

        for (v, f) in t.iter().zip(fields.iter()) {
            match f {
                Field::Expression(expr) => {
                    let expected = expr.evaluate(p, self)?;
                    if *v != expected {
                        return Ok(false);
                    }
                }
                _ => {}
            }
        }

        return Ok(true);
    }
    fn update_mem(
        &self,
        pos: usize,
        fields: &Vec<Field>,
        memory: Vec<i32>,
        tuple_spaces: Vec<Vec<Vec<i32>>>,
        ts_index: &u32,
        p: &Program,
        remove: bool,
    ) -> (Memory, Vec<Vec<Vec<i32>>>) {
        let mut mem_copy = memory.clone();
        let mut ts_copy = tuple_spaces.clone();

        let tuple = if remove {
            ts_copy[*ts_index as usize].remove(pos)
        } else {
            ts_copy[*ts_index as usize][pos].clone()
        };
        for (v, f) in tuple.iter().zip(fields.iter()) {
            if let Field::Variable(var) = f {
                let index = p.variable_index(var.name()).unwrap();
                mem_copy[index as usize] = *v;
            }
        }

        (mem_copy, ts_copy)
    }

    fn match_tuple_space_type(
        &self,
        ts_type: &TupleSpaceType,
        ts_index: &u32,
        fields: &Vec<Field>,
        p: &Program,
        ptr: &InstrPtr,
        remove: bool,
    ) -> Result<
        Either<
            std::array::IntoIter<(Memory, Vec<Vec<Vec<Int>>>, InstrPtr), 1>,
            std::vec::IntoIter<(Memory, Vec<Vec<Vec<Int>>>, InstrPtr)>,
        >,
        StepError,
    > {
        let tuple_spaces = self.tuple_spaces.clone();
        let memory = self.memory.clone();

        match ts_type {
            TupleSpaceType::Random => {
                let mut results = Vec::new();

                for (pos, t) in tuple_spaces[*ts_index as usize].iter().enumerate() {
                    if self.matches(t, fields, p)? {
                        let (mem_copy, ts_copy) = self.update_mem(
                            pos,
                            fields,
                            memory.clone(),
                            tuple_spaces.clone(),
                            ts_index,
                            p,
                            remove,
                        );

                        results.push((mem_copy, ts_copy, ptr.bump()));
                    }
                }

                if results.is_empty() {
                    return Err(StepError::Stuck);
                }

                Ok(Either::Right(results.into_iter()))
            }
            TupleSpaceType::FIFO => {
                let mut found_pos = None;

                for (pos, t) in tuple_spaces[*ts_index as usize].iter().enumerate() {
                    if self.matches(t, fields, p)? {
                        found_pos = Some(pos);
                        break;
                    }
                }

                if let Some(pos) = found_pos {
                    let (new_mem, new_ts) = self.update_mem(
                        pos,
                        fields,
                        memory.clone(),
                        tuple_spaces.clone(),
                        ts_index,
                        p,
                        remove,
                    );
                    Ok(Either::Left([(new_mem, new_ts, ptr.bump())].into_iter()))
                } else {
                    return Err(StepError::Stuck);
                }
            }
            TupleSpaceType::LIFO => {
                let mut found_pos = None;

                for (pos, t) in tuple_spaces[*ts_index as usize].iter().enumerate().rev() {
                    if self.matches(t, fields, p)? {
                        found_pos = Some(pos);
                        break;
                    }
                }

                if let Some(pos) = found_pos {
                    let (new_mem, new_ts) = self.update_mem(
                        pos,
                        fields,
                        memory.clone(),
                        tuple_spaces.clone(),
                        ts_index,
                        p,
                        remove,
                    );
                    Ok(Either::Left([(new_mem, new_ts, ptr.bump())].into_iter()))
                } else {
                    return Err(StepError::Stuck);
                }
            }
            TupleSpaceType::Queue => {
                if let Some(t) = tuple_spaces[*ts_index as usize].first() {
                    if self.matches(t, fields, p)? {
                        let (new_mem, new_ts) = self.update_mem(
                            0,
                            fields,
                            memory.clone(),
                            tuple_spaces.clone(),
                            ts_index,
                            p,
                            remove,
                        );
                        return Ok(Either::Left([(new_mem, new_ts, ptr.bump())].into_iter()));
                    }
                }
                Err(StepError::Stuck)
            }
            TupleSpaceType::Stack => {
                if let Some(t) = tuple_spaces[*ts_index as usize].last() {
                    let pos = tuple_spaces[*ts_index as usize].len() - 1;
                    if self.matches(t, fields, p)? {
                        let (new_mem, new_ts) = self.update_mem(
                            pos,
                            fields,
                            memory.clone(),
                            tuple_spaces.clone(),
                            ts_index,
                            p,
                            remove,
                        );
                        return Ok(Either::Left([(new_mem, new_ts, ptr.bump())].into_iter()));
                    }
                }
                Err(StepError::Stuck)
            }
        }
    }

    fn eval_guard(&self, p: &Program, expr: &BExpr) -> Vec<(Memory, Vec<Vec<Vec<Int>>>)> {
        match expr {
            BExpr::OP(Operation::Get(t, f)) => {
                let ts_index = p.tuple_space_index(t.name()).unwrap();
                let ts_type = p.tuple_spaces[ts_index as usize].space_type.clone();
                self.match_tuple_space_type(&ts_type, &ts_index, f, p, &InstrPtr(0), true)
                    .map(|either| either.into_iter().map(|(m, ts, _)| (m, ts)).collect())
                    .unwrap_or_default()
            }
            BExpr::OP(Operation::Query(t, f)) => {
                let ts_index = p.tuple_space_index(t.name()).unwrap();
                let ts_type = p.tuple_spaces[ts_index as usize].space_type.clone();
                self.match_tuple_space_type(&ts_type, &ts_index, f, p, &InstrPtr(0), false)
                    .map(|either| either.into_iter().map(|(m, ts, _)| (m, ts)).collect())
                    .unwrap_or_default()
            }
            BExpr::OP(Operation::Put(t, args)) => {
                let ts_index = p.tuple_space_index(t.name()).unwrap();
                let ts_meta = &p.tuple_spaces[ts_index as usize];
                if let TupleSpaceSize::Finite(max) = ts_meta.size {
                    if self.tuple_spaces[ts_index as usize].len() >= max as usize {
                        return vec![];
                    }
                }
                let mut ts = self.tuple_spaces.clone();
                let values = args
                    .iter()
                    .map(|e| e.evaluate(p, self).unwrap_or(0))
                    .collect();
                ts[ts_index as usize].push(values);
                vec![(self.memory.clone(), ts)]
            }
            BExpr::Logic(l, LogicOp::And | LogicOp::Land, r) => self
                .eval_guard(p, l)
                .into_iter()
                .flat_map(|(m, ts)| {
                    State {
                        ptrs: self.ptrs.clone(),
                        memory: m,
                        tuple_spaces: ts,
                    }
                    .eval_guard(p, r)
                })
                .collect(),
            BExpr::Logic(l, LogicOp::Or | LogicOp::Lor, r) => {
                let left_res = self.eval_guard(p, l);

                if left_res.is_empty() {
                    self.eval_guard(p, r)
                } else {
                    let chained: Vec<_> = left_res
                        .iter()
                        .flat_map(|(m, ts)| {
                            State {
                                ptrs: self.ptrs.clone(),
                                memory: m.clone(),
                                tuple_spaces: ts.clone(),
                            }
                            .eval_guard(p, r)
                        })
                        .collect();

                    if !chained.is_empty() {
                        chained
                    } else {
                        left_res
                    }
                }
            }
            _ => {
                if expr.evaluate(p, self).unwrap_or(false) {
                    vec![(self.memory.clone(), self.tuple_spaces.clone())]
                } else {
                    vec![]
                }
            }
        }
    }

    fn step_at(
        &self,
        p: &Program,
        ptr: InstrPtr,
    ) -> Result<impl Iterator<Item = (Memory, Vec<Vec<Vec<Int>>>, InstrPtr)>, StepError> {
        match &p[ptr] {
            Instr::Nop => Ok(Either::Left(
                [(self.memory.clone(), self.tuple_spaces.clone(), ptr.bump())].into_iter(),
            )),
            Instr::Assign(v, e) => {
                let value = e.evaluate(p, self)?;
                let mut memory = self.memory.clone();
                memory[*v as usize] = value;
                Ok(Either::Left(
                    [(memory, self.tuple_spaces.clone(), ptr.bump())].into_iter(),
                ))
            }
            Instr::Branch { choices, otherwise } => {
                let mut valid = Vec::new();
                for (b, target) in choices {
                    for (mem, ts) in self.eval_guard(p, b) {
                        valid.push((mem, ts, *target));
                    }
                }
                if valid.is_empty() {
                    if let Some(target) = otherwise {
                        Ok(Either::Left(
                            [(self.memory.clone(), self.tuple_spaces.clone(), *target)].into_iter(),
                        ))
                    } else {
                        Err(StepError::Stuck)
                    }
                } else {
                    Ok(Either::Right(valid.into_iter()))
                }
            }
            Instr::Goto(target) => Ok(Either::Left(
                [(self.memory.clone(), self.tuple_spaces.clone(), *target)].into_iter(),
            )),
            Instr::Halt => Err(StepError::Halt),
            Instr::Put(ts_max_size, ts_index, args) => {
                let values: Vec<Int> = args
                    .iter()
                    .map(|e| e.evaluate(p, self).map(Int::from))
                    .collect::<Result<_, _>>()?;

                let mut tuple_spaces = self.tuple_spaces.clone();

                match ts_max_size {
                    TupleSpaceSize::Finite(max_size) => {
                        if tuple_spaces[*ts_index as usize].len() < *max_size as usize {
                            tuple_spaces[*ts_index as usize].push(values);
                        } else {
                            return Err(StepError::Stuck);
                        }
                    }
                    TupleSpaceSize::Infinite => {
                        tuple_spaces[*ts_index as usize].push(values);
                    }
                }

                Ok(Either::Left(
                    [(self.memory.clone(), tuple_spaces, ptr.bump())].into_iter(),
                ))
            }
            Instr::Get(ts_type, ts_index, fields) => {
                self.match_tuple_space_type(ts_type, ts_index, fields, p, &ptr, true)
            }
            Instr::Query(ts_type, ts_index, fields) => {
                self.match_tuple_space_type(ts_type, ts_index, fields, p, &ptr, false)
            }
        }
    }

    pub fn raw_id(&self) -> String {
        let vars = self.memory.iter().format(" ");
        format!("{}@{}", vars, self.ptrs.iter().map(|p| p.0).format("X"))
    }
    pub fn format<'a>(&'a self, p: &'a Program) -> StateFormat<'a> {
        StateFormat {
            state: self,
            program: p,
        }
    }
}

pub struct StateFormat<'a> {
    state: &'a State,
    program: &'a Program,
}

impl fmt::Display for StateFormat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut parts = Vec::new();

        for (value, var) in self.state.memory.iter().zip(&self.program.variables) {
            parts.push(format!("{var} = {value}"));
        }

        for (ts_meta, ts_values) in self
            .program
            .tuple_spaces
            .iter()
            .zip(&self.state.tuple_spaces)
        {
            let tuples_str = ts_values
                .iter()
                .map(|tuple| {
                    format!(
                        "({})",
                        tuple
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("{} = {{{}}}", ts_meta.name, tuples_str))
        }

        write!(f, "{}", parts.join(", "))
    }
}

impl AExpr {
    fn evaluate(&self, p: &Program, state: &State) -> Result<i32, StepError> {
        Ok(match self {
            AExpr::Number(n) => *n,
            AExpr::Reference(r) => {
                let index = p.variable_index(r.name()).unwrap();
                state.memory[index as usize]
            }
            AExpr::Binary(l, op, r) => {
                let l = l.evaluate(p, state)?;
                let r = r.evaluate(p, state)?;
                match op {
                    AOp::Plus => l + r,
                    AOp::Minus => l - r,
                    AOp::Times => l * r,
                    AOp::Divide => l / r,
                }
            }
            AExpr::Minus(e) => -e.evaluate(p, state)?,
            AExpr::Function(f) => match f {
                Function::Division(a, b) => {
                    let a = a.evaluate(p, state)?;
                    let b = b.evaluate(p, state)?;
                    if b == 0 {
                        return Err(StepError::DivisionByZero);
                    }
                    a / b
                }
                Function::Min(a, b) => {
                    let a = a.evaluate(p, state)?;
                    let b = b.evaluate(p, state)?;
                    a.min(b)
                }
                Function::Max(a, b) => {
                    let a = a.evaluate(p, state)?;
                    let b = b.evaluate(p, state)?;
                    a.max(b)
                }
                Function::Fac(x) => {
                    let x = x.evaluate(p, state)?;
                    if x < 0 {
                        return Err(StepError::NegativeFactorial);
                    }
                    (1..=x).product()
                }
                Function::Fib(x) => {
                    let x = x.evaluate(p, state)?;
                    if x < 0 {
                        return Err(StepError::NegativeFibonacci);
                    }
                    let mut a = 0;
                    let mut b = 1;
                    for _ in 0..x {
                        let c = a + b;
                        a = b;
                        b = c;
                    }
                    a
                }
                Function::Exp(a, b) => {
                    let a = a.evaluate(p, state)?;
                    let b = b.evaluate(p, state)?;
                    if b < 0 {
                        return Err(StepError::NegativePower);
                    }
                    a.pow(b as u32)
                }
            },
            AExpr::Old(_) => return Err(StepError::HitOld),
        })
    }
}

impl BExpr {
    pub fn evaluate(&self, p: &Program, state: &State) -> Result<bool, StepError> {
        Ok(match self {
            BExpr::Bool(b) => *b,
            BExpr::Rel(l, op, r) => {
                let l = l.evaluate(p, state)?;
                let r = r.evaluate(p, state)?;
                match op {
                    RelOp::Eq => l == r,
                    RelOp::Ne => l != r,
                    RelOp::Lt => l < r,
                    RelOp::Le => l <= r,
                    RelOp::Gt => l > r,
                    RelOp::Ge => l >= r,
                }
            }
            BExpr::Logic(l, op, r) => {
                let l = l.evaluate(p, state)?;
                let r = r.evaluate(p, state)?;
                match op {
                    LogicOp::And => l && r,
                    LogicOp::Land => l && r,
                    LogicOp::Or => l || r,
                    LogicOp::Lor => l || r,
                    LogicOp::Implies => !l || r,
                }
            }
            BExpr::Not(e) => !e.evaluate(p, state)?,
            BExpr::Quantified(_, _, _) => todo!(),
            BExpr::OP(_) => !state.eval_guard(p, self).is_empty(),
        })
    }
}

impl LTLFormula {
    pub fn to_mcltl(
        &self,
        rels: &mut Vec<(AExpr, RelOp, AExpr)>,
        operations: &mut Vec<Operation>,
    ) -> mcltl::ltl::expression::LTLExpression {
        use mcltl::ltl::expression::LTLExpression;

        match self {
            LTLFormula::Bool(true) => LTLExpression::True,
            LTLFormula::Bool(false) => LTLExpression::False,
            LTLFormula::Locator(l) => LTLExpression::lit(l.to_lit()),
            LTLFormula::Rel(lhs, op, rhs) => {
                let idx = if let Some(idx) = rels
                    .iter()
                    .position(|(l, o, r)| l == lhs && o == op && r == rhs)
                {
                    idx
                } else {
                    rels.push((lhs.clone(), *op, rhs.clone()));
                    rels.len() - 1
                };
                LTLExpression::Literal(format!("p{idx}").into())
            }
            LTLFormula::Operation(op) => {
                let idx = if let Some(idx) = operations
                    .iter()
                    .position(|x| x == op.as_ref())
                {
                    idx
                } else {
                    operations.push(op.as_ref().clone());
                    operations.len() - 1
                };
                LTLExpression::Literal(format!("o{idx}").into())
            }
            LTLFormula::Not(e) => !e.to_mcltl(rels, operations),
            LTLFormula::And(p, q) => p.to_mcltl(rels, operations) & q.to_mcltl(rels, operations),
            LTLFormula::Or(p, q) => p.to_mcltl(rels, operations) | q.to_mcltl(rels, operations),
            LTLFormula::Implies(p, q) => !p.to_mcltl(rels, operations) | q.to_mcltl(rels, operations),
            LTLFormula::Until(p, q) => p.to_mcltl(rels, operations).U(q.to_mcltl(rels, operations)),
            LTLFormula::Next(p) => LTLExpression::X(Box::new(p.to_mcltl(rels, operations))),
            LTLFormula::Globally(p) => LTLExpression::G(Box::new(p.to_mcltl(rels, operations))),
            LTLFormula::Finally(p) => LTLExpression::F(Box::new(p.to_mcltl(rels, operations))),
        }
    }
}

impl Locator {
    pub fn to_lit(&self) -> Literal {
        format!("@{self}").into()
    }
}
