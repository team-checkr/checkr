use itertools::Itertools;

use crate::{
    ast::{
        AExpr, AOp, BExpr, Command, CommandKind, Commands, Function, LTLFormula, LogicOp, RelOp,
        Target, Variable,
    },
    ast_ext::FreeVariables,
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
        };

        for cmds in cmdss {
            let entry = p.current();
            p.entry_points.push(entry);
            p.compile_commands(cmds);
            p.push(Instr::Halt);
        }

        p
    }

    pub fn initial_state(&self, memory: impl Fn(&Variable) -> i32) -> State {
        State {
            ptrs: self.entry_points.clone(),
            memory: self.variables.iter().map(memory).collect(),
        }
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

    fn push(&mut self, instr: Instr) -> InstrPtr {
        let ptr = self.current();
        self.instrs.push(instr);
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
                self.push(Instr::Assign(index, e.clone()));
            }
            CommandKind::Skip => {
                self.push(Instr::Nop);
            }
            CommandKind::If(guards) => {
                let head = self.push(Instr::Nop);
                let mut choices = Vec::new();
                let mut exits = Vec::new();
                for guard in guards {
                    choices.push((guard.guard.clone(), self.current()));
                    self.compile_commands(&guard.cmds);
                    exits.push(self.current());
                    self.push(Instr::Nop);
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
                let head = self.push(Instr::Nop);
                let mut choices = Vec::new();
                for guard in guards {
                    choices.push((guard.guard.clone(), self.current()));
                    self.compile_commands(&guard.cmds);
                    self.push(Instr::Goto(head));
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
}

impl State {
    pub fn step(&self, p: &Program) -> Vec<Result<State, StepError>> {
        self.ptrs
            .iter()
            .enumerate()
            .map(|(i, _)| self.step_exe(p, i))
            .collect()
    }
    fn step_exe(&self, p: &Program, execution: usize) -> Result<State, StepError> {
        let (mem, ptr) = self.step_at(p, self.ptrs[execution])?;
        let mut ptrs = self.ptrs.clone();
        ptrs[execution] = ptr;
        Ok(State { ptrs, memory: mem })
    }
    fn step_at(&self, p: &Program, ptr: InstrPtr) -> Result<(Memory, InstrPtr), StepError> {
        match &p[ptr] {
            Instr::Nop => Ok((self.memory.clone(), ptr.bump())),
            Instr::Assign(v, e) => {
                let value = e.evaluate(p, self)?;
                let mut memory = self.memory.clone();
                memory[*v as usize] = value;
                Ok((memory, ptr.bump()))
            }
            Instr::Branch { choices, otherwise } => {
                for (b, target) in choices {
                    if b.evaluate(p, self)? {
                        return Ok((self.memory.clone(), *target));
                    }
                }
                if let Some(target) = otherwise {
                    Ok((self.memory.clone(), *target))
                } else {
                    Err(StepError::Stuck)
                }
            }
            Instr::Goto(target) => Ok((self.memory.clone(), *target)),
            Instr::Halt => Err(StepError::Halt),
        }
    }

    pub fn raw_id(&self) -> String {
        let vars = self.memory.iter().format(" ");
        format!("{}@{}", vars, self.ptrs.iter().map(|p| p.0).format("X"))
    }
    pub fn id(&self, p: &Program) -> String {
        let vars = self
            .memory
            .iter()
            .zip(&p.variables)
            .map(|(value, name)| format!("{name}{value}"))
            .format("");
        format!("{}X{}", vars, self.ptrs.iter().map(|p| p.0).format("X"))
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
            // Bool(_), Rel(_, _, _), Not(_), And(_, _), Or(_, _), Implies(_, _), Until(_, _), Next(_), Globally(_), Finally(_),
            LTLFormula::Bool(b) => {
                if *b {
                    LTLExpression::True
                } else {
                    LTLExpression::False
                }
            }
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
                LTLExpression::Literal(format!("p{idx}"))
            }
            LTLFormula::Not(e) => LTLExpression::Not(Box::new(e.to_mcltl(rels))),
            LTLFormula::And(p, q) => {
                LTLExpression::And(Box::new(p.to_mcltl(rels)), Box::new(q.to_mcltl(rels)))
            }
            LTLFormula::Or(p, q) => {
                LTLExpression::Or(Box::new(p.to_mcltl(rels)), Box::new(q.to_mcltl(rels)))
            }
            LTLFormula::Implies(p, q) => {
                todo!();
                // LTLExpression::Implies(Box::new(p.to_mcltl(rels)), Box::new(q.to_mcltl(rels)))
            }
            LTLFormula::Until(p, q) => {
                LTLExpression::U(Box::new(p.to_mcltl(rels)), Box::new(q.to_mcltl(rels)))
            }
            LTLFormula::Next(p) => {
                todo!();
                // LTLExpression::Next(Box::new(p.to_mcltl(rels)))
            }
            LTLFormula::Globally(p) => LTLExpression::G(Box::new(p.to_mcltl(rels))),
            LTLFormula::Finally(p) => LTLExpression::F(Box::new(p.to_mcltl(rels))),
        }
    }
}
