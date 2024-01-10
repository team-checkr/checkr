use dioxus::prelude::*;
use gcl::ast::{BExpr, Command, Commands};
use rand::SeedableRng;

use crate::{components::MonacoEditor, Generate};

#[inline_props]
pub fn GclEditor<'a>(
    cx: Scope<'a>,
    commands: Commands,
    on_change: EventHandler<'a, Commands>,
) -> Element {
    let last_parsed_commands = use_state(cx, || commands.clone());
    let internal_value = use_state(cx, || commands.to_string());

    use_effect(cx, (commands,), |(commands,)| {
        to_owned![last_parsed_commands, internal_value];
        async move {
            if last_parsed_commands.get() != &commands {
                last_parsed_commands.set(commands.clone());
                internal_value.set(commands.to_string());
            }
        }
    });

    cx.render(rsx!(
        div {
            class: "grid grid-rows-[auto_1fr]",
            div {
                class: "flex border-b divide-x",
                button {
                    class: "px-2 py-1",
                    onclick: move |_| on_change.call(regenerate_commands()),
                    "Regenerate"
                }
            }
            MonacoEditor {
                value: internal_value.get().to_string(),
                on_change: move |value: String| {
                    if let Ok(cmds) = gcl::parse::parse_commands(&value) {
                        last_parsed_commands.set(cmds.clone());
                        on_change.call(cmds);
                    }
                },
            }
        }
    ))
}

fn regenerate_commands() -> Commands {
    Commands::gen(
        &mut Default::default(),
        &mut rand::rngs::SmallRng::from_entropy(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnnotatedCommand {
    pub pre: BExpr,
    pub cmds: Commands,
    pub post: BExpr,
}

impl std::fmt::Display for AnnotatedCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Command::Annotated(self.pre.clone(), self.cmds.clone(), self.post.clone()).fmt(f)
    }
}

#[inline_props]
pub fn GclAnnotatedEditor<'a>(
    cx: Scope<'a>,
    command: AnnotatedCommand,
    on_change: EventHandler<'a, AnnotatedCommand>,
) -> Element {
    let last_parsed_command = use_state(cx, || command.clone());
    let internal_value = use_state(cx, || command.to_string());

    use_effect(cx, (command,), |(command,)| {
        to_owned![last_parsed_command, internal_value];
        async move {
            if last_parsed_command.get() != &command {
                last_parsed_command.set(command.clone());
                internal_value.set(command.to_string());
            }
        }
    });

    cx.render(rsx!(
        div {
            class: "grid grid-rows-[auto_1fr]",
            div {
                class: "flex border-b divide-x",
                button {
                    class: "px-2 py-1",
                    onclick: move |_| on_change.call(regenerate_annotated_command()),
                    "Regenerate"
                }
            }
            MonacoEditor {
                value: internal_value.get().to_string(),
                on_change: move |value: String| {
                    if let Ok(Command::Annotated(pre, cmds, post)) = gcl::parse::parse_annotated_command(&value) {
                        let cmds = AnnotatedCommand { pre, cmds, post };
                        last_parsed_command.set(cmds.clone());
                        on_change.call(cmds);
                    }
                },
            }
        }
    ))
}

fn regenerate_annotated_command() -> AnnotatedCommand {
    let cmds = Commands::gen(
        &mut Default::default(),
        &mut rand::rngs::SmallRng::from_entropy(),
    );

    AnnotatedCommand {
        pre: BExpr::Bool(true),
        cmds,
        post: BExpr::Bool(true),
    }
}
