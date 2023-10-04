use dioxus::prelude::*;

use crate::ValidationResult;

#[derive(Props)]
pub struct StandardLayoutProps<'a> {
    pub input: Element<'a>,
    pub output: Element<'a>,
}

pub fn StandardLayout<'a>(cx: Scope<'a, StandardLayoutProps<'a>>) -> Element<'a> {
    let validation_result = use_shared_state::<Option<ValidationResult>>(cx).unwrap();

    let color = match &*validation_result.read() {
        Some(ValidationResult::CorrectTerminated)
        | Some(ValidationResult::CorrectNonTerminated { .. }) => "bg-correct",
        Some(ValidationResult::Mismatch { .. }) => "bg-mismatch",
        Some(ValidationResult::TimeOut) => "bg-time-out",
        None => "bg-working",
    };

    cx.render(rsx!(div {
        class: "grid grid-cols-[45ch_1fr] grid-rows-[1fr_auto]",
        div {
            class: "border-r relative row-span-2",
            div {
                class: "absolute inset-0 grid overflow-auto",
                &cx.props.input
            }
        }
        div {
            class: "relative",
            div {
                class: "absolute inset-0 grid overflow-auto",
                &cx.props.output
            }
        }
        div {
            class: "h-4 transition {color}"
        }
    }))
}
