use dioxus::prelude::*;
use gcl::ast::Commands;
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
