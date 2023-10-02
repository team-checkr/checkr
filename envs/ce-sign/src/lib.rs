#![allow(non_snake_case)]

mod semantics;

use std::{collections::HashSet, fmt::Display, hash::Hash};

use ce_core::{
    components::{GclEditor, Network, StandardLayout},
    rand::{self, seq::SliceRandom, SeedableRng},
    Env, EnvError, Generate, RenderProps, ValidationResult,
};
use dioxus::{self, prelude::*};
use gcl::{
    ast::{Array, Commands, Target, Variable},
    memory::{Memory, MemoryRef},
    pg::{
        analysis::{mono_analysis, FiFo},
        Determinism, Node, ProgramGraph,
    },
};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub use semantics::{Bools, Sign, SignAnalysis, SignMemory, Signs};

#[derive(Debug, Default)]
pub struct SignEnv;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignInput {
    pub commands: Commands,
    pub determinism: Determinism,
    pub assignment: SignMemory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignOutput {
    pub initial_node: String,
    pub final_node: String,
    pub nodes: IndexMap<String, HashSet<SignMemory>>,
}

impl Env for SignEnv {
    type Input = SignInput;

    type Output = SignOutput;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let pg = ProgramGraph::new(input.determinism, &input.commands);

        for t in pg.fv() {
            match t {
                Target::Variable(var) => {
                    if input.assignment.get_var(&var).is_none() {
                        return Err(EnvError::InvalidInputForProgram {
                            message: format!("variable `{var}` was not in the given input"),
                        });
                    }
                }
                Target::Array(arr, _) => {
                    if input.assignment.get_arr(&arr).is_none() {
                        return Err(EnvError::InvalidInputForProgram {
                            message: format!("array `{arr}` was not in the given input"),
                        });
                    }
                }
            }
        }

        let nodes = mono_analysis::<_, FiFo>(
            SignAnalysis {
                assignment: input.assignment.clone(),
            },
            &pg,
        )
        .facts
        .into_iter()
        .map(|(k, v)| (format!("{k}"), v))
        .collect();
        Ok(SignOutput {
            initial_node: Node::Start.to_string(),
            final_node: Node::End.to_string(),
            nodes,
        })
    }

    fn validate(
        input: &Self::Input,
        output: &Self::Output,
    ) -> ce_core::Result<ce_core::ValidationResult> {
        let reference = Self::run(input)?;

        let mut pool = reference.nodes.values().collect_vec();

        for (n, o) in &output.nodes {
            if let Some(idx) = pool.iter().position(|r| *r == o) {
                pool.remove(idx);
            } else {
                tracing::error!(not_in_reference = format!("{o:?}"), "damn...");
                return Ok(ValidationResult::Mismatch {
                    reason: format!(
                        "Produced world which did not exist in reference: {n:?} ~> {o:?}"
                    ),
                });
            }
        }

        if pool.is_empty() {
            Ok(ValidationResult::CorrectTerminated)
        } else {
            tracing::error!(missing = format!("{pool:?}"), "oh no...");
            Ok(ValidationResult::Mismatch {
                reason: "Reference had world which was not present".to_string(),
            })
        }
    }

    fn render<'a>(cx: &'a ScopeState, props: &'a RenderProps<'a, Self>) -> Element<'a> {
        let reference_dot = ProgramGraph::new(props.input.determinism, &props.input.commands).dot();
        let real_dot = ProgramGraph::new(props.input.determinism, &props.input.commands).dot();

        cx.render(rsx!(
            StandardLayout {
                input: cx.render(rsx!(div {
                    class: "grid grid-rows-2",
                    GclEditor {
                        commands: props.input.commands.clone(),
                        on_change: move |commands| props.set_input(SignInput::gen_from_commands(&mut rand::rngs::SmallRng::from_entropy(), commands)),
                    }
                    div {
                        class: "grid grid-cols-4",
                        for thingy in props.input.assignment.iter() {
                            match thingy {
                                MemoryRef::Variable(name, &sign) => cx.render(rsx!(
                                    span { "{name}" }
                                    for s in [Sign::Negative, Sign::Zero, Sign::Positive] {
                                        input {
                                            r#type: "radio",
                                            checked: sign == s,
                                            onclick: move |_| props.set_input(props.input.set_sign(name, s)),
                                        }
                                    }
                                )),
                                MemoryRef::Array(name, &signs) => cx.render(rsx!(
                                    span { "{name}" }
                                    for s in [Sign::Negative, Sign::Zero, Sign::Positive] {
                                        input {
                                            r#type: "checkbox",
                                            checked: signs.contains(s.into()),
                                            onclick: move |_| props.set_input(props.input.set_signs(name, signs ^ s.into())),
                                        }
                                    }
                                )),
                            }
                        }
                    }
                })),
                output: cx.render(rsx!(div {
                    class: "grid grid-rows-[auto_1fr_1fr] grid-cols-2",
                    div { "Real" }
                    div { "Reference" }
                    div {
                        class: "grid relative border-b border-r",
                        div {
                            class: "absolute inset-0 grid",
                            Network { dot: real_dot }
                        }
                    }
                    div {
                        class: "grid relative border-b",
                        div {
                            class: "absolute inset-0 grid",
                            Network { dot: reference_dot }
                        }
                    }
                    div {
                        class: "relative border-r",
                        div {
                            class: "absolute inset-0 overflow-auto",
                            ViewThingy { thingy: props.real_output.nodes.clone() }
                        }
                    }
                    div {
                        class: "relative",
                        div {
                            class: "absolute inset-0 overflow-auto",
                            ViewThingy { thingy: props.reference_output.nodes.clone() }
                        }
                    }
                })),
            }
        ))
    }
}

impl SignInput {
    fn set_sign(&self, var: &Variable, sign: Sign) -> SignInput {
        let mut new = self.clone();
        new.assignment.variables.insert(var.clone(), sign);
        new
    }
    fn set_signs(&self, arr: &Array, signs: Signs) -> SignInput {
        let mut new = self.clone();
        new.assignment.arrays.insert(arr.clone(), signs);
        new
    }
}

#[inline_props]
fn ViewThingy<A: Display + PartialEq + Eq + Hash, B: Display + PartialEq + Eq + Hash>(
    cx: Scope,
    thingy: IndexMap<String, HashSet<Memory<A, B>>>,
) -> Element {
    let vars = thingy
        .values()
        .flat_map(|mems| {
            mems.iter()
                .flat_map(|mem| mem.iter().map(|mem_ref| mem_ref.target()))
        })
        .sorted()
        .dedup()
        .collect_vec();

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum CharClass<A, T, O> {
        Start,
        Alpha(A),
        Numeric(T),
        Other(O),
        End,
    }

    let nodes = thingy.iter().sorted_by_key(|(name, _)| {
        name.chars()
            .group_by(|&c| {
                if c.is_alphabetic() {
                    CharClass::Alpha(())
                } else if c.is_numeric() {
                    CharClass::Numeric(())
                } else if c == '▷' {
                    CharClass::Start
                } else if c == '◀' {
                    CharClass::End
                } else {
                    CharClass::Other(())
                }
            })
            .into_iter()
            .map(|(key, chars)| match key {
                CharClass::Start => CharClass::Start,
                CharClass::Alpha(()) => CharClass::Alpha(chars.into_iter().join("")),
                CharClass::Numeric(()) => CharClass::Numeric(
                    chars
                        .into_iter()
                        .join("")
                        .parse::<i64>()
                        .unwrap_or(i64::MAX),
                ),
                CharClass::Other(()) => CharClass::Other(chars.into_iter().join("")),
                CharClass::End => CharClass::End,
            })
            .collect_vec()
    });

    cx.render(rsx!(div {
        class: "flex items-start",
        div {
            class: "[&_*]:border-t grid grid-flow-dense w-full",
            style: "grid-template-columns: repeat({vars.len() + 1}, auto);",
            div { class: "border-none" }
            for target in &vars {
                div { class: "text-center border-none", "{target}" }
            }
            for (name, mems) in nodes {
                for (idx, mem) in mems.iter().enumerate() {
                    if idx == 0 {
                        rsx!(h2 {
                            class: "px-2",
                            style: "grid-row: span {mems.len()} / span {mems.len()};",
                            "{name}"
                        })
                    }
                    for thingy in mem.iter() {
                        div {
                            class: "text-sm font-mono px-2 py-0.5",
                            "{thingy}"
                        }
                    }
                }
            }
        }
    }))
}

impl SignInput {
    fn gen_from_commands<R: rand::Rng>(rng: &mut R, commands: Commands) -> SignInput {
        let assignment = SignMemory::from_targets_with(
            commands.fv(),
            rng,
            |rng, _| Generate::gen(&mut (), rng),
            |rng, _| Generate::gen(&mut (), rng),
        );

        SignInput {
            commands,
            assignment,
            determinism: Determinism::Deterministic,
        }
    }
}

impl Generate for SignInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        let commands = Commands::gen(&mut Default::default(), rng);
        SignInput::gen_from_commands(rng, commands)
    }
}

impl Generate for Sign {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        *[Sign::Positive, Sign::Zero, Sign::Negative]
            .choose(rng)
            .unwrap()
    }
}
impl Generate for Signs {
    type Context = ();

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        [Sign::gen(cx, rng)].into_iter().collect()
    }
}
