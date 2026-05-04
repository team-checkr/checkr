use std::collections::HashMap;

use itertools::Either;
use la_arena::{Arena, ArenaMap, Idx};

use crate::{
    CallNumber, Label, ProgramPoint, Reg, RiscVFile, RiscVVMDisplay, StepResult, Word,
    instr::Instruction,
};

type Instr0 = Instruction<Idx<Reg>, Idx<Label>, Idx<Label>>;
type Instr = Instruction<Idx<Reg>, Word, ProgramPoint>;

#[derive(Debug, Default)]
pub struct Binary {
    labels: Arena<Label>,
    regs: Arena<Reg>,
    labels_map: HashMap<Label, Idx<Label>>,
    regs_map: HashMap<Reg, Idx<Reg>>,
    label_mem: ArenaMap<Idx<Label>, Word>,
    label_jmp: ArenaMap<Idx<Label>, ProgramPoint>,
    instr: Vec<Instr>,

    default_regs: DefaultRegs,
}

#[derive(Debug)]
struct DefaultRegs {
    a7: Idx<Reg>,
}

impl Default for DefaultRegs {
    fn default() -> Self {
        Self {
            a7: Idx::from_raw(la_arena::RawIdx::from_u32(7)),
        }
    }
}

pub struct VM {
    pub(crate) pc: ProgramPoint,
    pub(crate) memory: Memory,
}

pub struct Memory {
    regs: ArenaMap<Idx<Reg>, Word>,
    heap: Vec<Word>,
}

impl std::ops::Index<Word> for Memory {
    type Output = Word;

    fn index(&self, index: Word) -> &Self::Output {
        &self.heap[index.0 as usize]
    }
}
impl std::ops::IndexMut<Word> for Memory {
    fn index_mut(&mut self, index: Word) -> &mut Self::Output {
        &mut self.heap[index.0 as usize]
    }
}
impl Memory {
    fn reg(&self, reg: Idx<Reg>) -> Word {
        self.regs.get(reg).copied().unwrap_or_default()
    }
}

impl Binary {
    fn label_mem(&self, idx: Idx<Label>) -> Word {
        self.label_mem.get(idx).copied().unwrap_or_default()
    }
    fn label_jmp(&self, idx: Idx<Label>) -> ProgramPoint {
        self.label_jmp.get(idx).copied().unwrap_or_default()
    }
    fn label(&mut self, label: Label) -> Idx<Label> {
        *self
            .labels_map
            .entry(label)
            .or_insert_with_key(|label| self.labels.alloc(label.clone()))
    }
    fn reg(&mut self, reg: Reg) -> Idx<Reg> {
        *self
            .regs_map
            .entry(reg)
            .or_insert_with_key(|reg| self.regs.alloc(reg.clone()))
    }

    pub fn from_file(f: &RiscVFile) -> (Binary, Memory) {
        let mut bin = Binary::default();
        let mut mem = Memory {
            heap: vec![Word(0); 2_usize.pow(16)],
            regs: Default::default(),
        };

        bin.default_regs.a7 = bin.reg(Reg::a7());

        for (lbl, init) in &f.data {
            let idx = bin.label(lbl.clone());
            let mem_loc = Word(idx.into_raw().into_u32() as _);
            bin.label_mem.insert(idx, mem_loc);
            mem[mem_loc] = *init;
        }

        let mut instr0: Vec<Instr0> = Vec::with_capacity(f.text.len());

        for item in &f.text {
            match item {
                Either::Left(lbl) => {
                    let idx = bin.label(lbl.clone());
                    bin.label_jmp.insert(idx, ProgramPoint(instr0.len() as _));
                }
                Either::Right(inst) => {
                    let inst = inst
                        .clone()
                        .map(|reg| bin.reg(reg), |label| label, |pp| pp)
                        .map(|reg| reg, |label| bin.label(label), |pp| pp)
                        .map(|reg| reg, |label| label, |pp| bin.label(pp));
                    instr0.push(inst);
                }
            }
        }

        bin.instr = instr0
            .into_iter()
            .map(|inst| inst.map(|reg| reg, |lbl| bin.label_mem(lbl), |pp| bin.label_jmp(pp)))
            .collect();

        (bin, mem)
    }

    fn inst(&self, pc: ProgramPoint) -> Option<Instr> {
        self.instr.get(pc.0 as usize).copied()
    }
}

impl VM {
    pub fn new(init_mem: Memory) -> VM {
        VM {
            pc: ProgramPoint(0),
            memory: init_mem,
        }
    }

    pub fn display(&self, bin: &Binary) -> RiscVVMDisplay {
        let last_non_zero = self
            .memory
            .heap
            .iter()
            .rposition(|&w| w != Word(0))
            .unwrap_or_default();
        RiscVVMDisplay {
            pc: self.pc.0,
            regs: bin
                .regs_map
                .iter()
                .map(|(reg, idx)| (reg.to_string(), self.memory.reg(*idx)))
                .collect(),
            variables: bin
                .label_mem
                .iter()
                .map(|(lbl, &word)| (bin.labels[lbl].to_string(), (word, self.memory[word])))
                .collect(),
            memory: self.memory.heap[0..=last_non_zero].to_vec(),
        }
    }

    pub fn run(&mut self, bin: &Binary, steps: usize) -> StepResult {
        for _ in 0..steps {
            match self.step(bin) {
                StepResult::Exit => return StepResult::Exit,
                StepResult::Stuck => return StepResult::Stuck,
                StepResult::Ok => {}
            }
        }
        StepResult::Ok
    }

    fn step(&mut self, bin: &Binary) -> StepResult {
        if let Some(inst) = bin.inst(self.pc) {
            self.pc = self.pc.inc();

            match inst {
                Instruction::li(reg, word) => {
                    self.memory.regs.insert(reg, word);
                }
                Instruction::lw(reg, label) => {
                    self.memory.regs.insert(reg, self.memory[label]);
                }
                Instruction::la(reg, label) => {
                    self.memory.regs.insert(reg, label);
                }
                Instruction::mv(reg, reg1) => {
                    self.memory.regs.insert(reg, self.memory.regs[reg1]);
                }
                Instruction::sw(reg, o, reg1) => {
                    let idx = o + self.memory.regs[reg1];
                    self.memory[idx] = self.memory.regs[reg];
                }
                Instruction::add(reg, reg1, reg2) => {
                    self.memory
                        .regs
                        .insert(reg, self.memory.regs[reg1] + self.memory.regs[reg2]);
                }
                Instruction::neg(reg, reg1) => {
                    self.memory.regs.insert(reg, -self.memory.regs[reg1]);
                }
                Instruction::sub(reg, reg1, reg2) => {
                    self.memory
                        .regs
                        .insert(reg, self.memory.regs[reg1] - self.memory.regs[reg2]);
                }
                Instruction::mul(reg, reg1, reg2) => {
                    self.memory
                        .regs
                        .insert(reg, self.memory.regs[reg1] * self.memory.regs[reg2]);
                }
                Instruction::div(reg, reg1, reg2) => {
                    let r = self.memory.regs[reg2];
                    if r == Word(0) {
                        return StepResult::Stuck;
                    } else {
                        self.memory.regs[reg] = self.memory.regs[reg1] / r;
                    }
                }
                Instruction::j(pc) => {
                    self.pc = pc;
                }
                Instruction::beq(reg, reg1, pc) => {
                    if self.memory.regs[reg] == self.memory.regs[reg1] {
                        self.pc = pc;
                    }
                }
                Instruction::bne(reg, reg1, pc) => {
                    if self.memory.regs[reg] != self.memory.regs[reg1] {
                        self.pc = pc;
                    }
                }
                Instruction::blt(reg, reg1, pc) => {
                    if self.memory.regs[reg] < self.memory.regs[reg1] {
                        self.pc = pc;
                    }
                }
                Instruction::ebreak => return StepResult::Stuck,
                Instruction::ecall => match self.memory.regs[bin.default_regs.a7] {
                    CallNumber::EXIT => {
                        return StepResult::Exit;
                    }
                    _ => return StepResult::Stuck,
                },
            }

            StepResult::Ok
        } else {
            StepResult::Stuck
        }
    }
}
