use dioxus::prelude::*;

#[derive(Props)]
pub struct StandardLayoutProps<'a> {
    pub input: Element<'a>,
    pub output: Element<'a>,
}

pub fn StandardLayout<'a>(cx: Scope<'a, StandardLayoutProps<'a>>) -> Element<'a> {
    cx.render(rsx!(div {
        class: "grid grid-cols-[45ch_1fr]",
        div {
            class: "border-r relative",
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
    }))
}
