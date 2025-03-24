use std::fmt;

use itertools::{Either, Itertools};
use mcltl::ltl::expression::Literal;

use crate::{
    ast::{
        AExpr, AOp, BExpr, Command, CommandKind, Commands, Function, LTLFormula, Locator, LogicOp,
        RelOp, Target, Variable,
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
}

#[derive(Debug)]
pub struct Program {
    variables: Vec<Variable>,
    instrs: Vec<Instr>,
    entry_points: Vec<InstrPtr>,
    source_map: Vec<Option<SourceSpan>>,
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

    pub fn initial_state(&self, memory: impl Fn(&Variable) -> i32) -> State {
        State {
            ptrs: self.entry_points.clone(),
            memory: self.variables.iter().map(memory).collect(),
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
        }
    }
}

type Memory = Vec<i32>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    ptrs: Vec<InstrPtr>,
    memory: Memory,
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
    fn step_exe(
        &self,
        p: &Program,
        execution: usize,
    ) -> Result<impl Iterator<Item = State> + '_, StepError> {
        Ok(self
            .step_at(p, self.ptrs[execution])?
            .map(move |(mem, ptr)| {
                let mut ptrs = self.ptrs.clone();
                ptrs[execution] = ptr;
                State { ptrs, memory: mem }
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
    fn step_at(
        &self,
        p: &Program,
        ptr: InstrPtr,
    ) -> Result<impl Iterator<Item = (Memory, InstrPtr)>, StepError> {
        match &p[ptr] {
            Instr::Nop => Ok(Either::Left(
                [(self.memory.clone(), ptr.bump())].into_iter(),
            )),
            Instr::Assign(v, e) => {
                let value = e.evaluate(p, self)?;
                let mut memory = self.memory.clone();
                memory[*v as usize] = value;
                Ok(Either::Left([(memory, ptr.bump())].into_iter()))
            }
            Instr::Branch { choices, otherwise } => {
                let mut valid = Vec::new();
                for (b, target) in choices {
                    if b.evaluate(p, self)? {
                        valid.push((self.memory.clone(), *target));
                    }
                }
                if valid.is_empty() {
                    if let Some(target) = otherwise {
                        Ok(Either::Left([(self.memory.clone(), *target)].into_iter()))
                    } else {
                        Err(StepError::Stuck)
                    }
                } else {
                    Ok(Either::Right(valid.into_iter()))
                }
            }
            Instr::Goto(target) => Ok(Either::Left([(self.memory.clone(), *target)].into_iter())),
            Instr::Halt => Err(StepError::Halt),
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
        write!(
            f,
            "{}",
            self.state
                .memory
                .iter()
                .zip(&self.program.variables)
                .map(|(value, var)| format!("{var} = {value}"))
                .format(", ")
        )
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
        })
    }
}

impl LTLFormula {
    pub fn to_mcltl(
        &self,
        rels: &mut Vec<(AExpr, RelOp, AExpr)>,
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
            LTLFormula::Not(e) => !e.to_mcltl(rels),
            LTLFormula::And(p, q) => p.to_mcltl(rels) & q.to_mcltl(rels),
            LTLFormula::Or(p, q) => p.to_mcltl(rels) | q.to_mcltl(rels),
            LTLFormula::Implies(p, q) => !p.to_mcltl(rels) | q.to_mcltl(rels),
            LTLFormula::Until(p, q) => p.to_mcltl(rels).U(q.to_mcltl(rels)),
            LTLFormula::Next(p) => LTLExpression::X(Box::new(p.to_mcltl(rels))),
            LTLFormula::Globally(p) => LTLExpression::G(Box::new(p.to_mcltl(rels))),
            LTLFormula::Finally(p) => LTLExpression::F(Box::new(p.to_mcltl(rels))),
        }
    }
}

impl Locator {
    pub fn to_lit(&self) -> Literal {
        format!("@{self}").into()
    }
}
