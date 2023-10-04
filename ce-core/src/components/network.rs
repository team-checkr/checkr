use dioxus::prelude::*;
use tracing::Instrument;

#[derive(Props, PartialEq)]
pub struct NetworkProps {
    pub dot: String,
}

#[tracing::instrument(skip_all, fields(cx.props.dot))]
pub fn Network(cx: Scope<NetworkProps>) -> Element {
    let create_eval = use_eval(cx);

    let id = cx.raw_text(format_args!("viz-network-{}", cx.scope_id().0));

    use_effect(cx, (&id.to_string(), &cx.props.dot), |(id, dot)| {
        let eval = create_eval(
            &include_str!("./network.js")
                .replace("%id%", &id)
                .replace("\"%dot%\"", &serde_json::to_string(&dot).unwrap()),
        )
        .unwrap();

        async move {
            tracing::info!("sending id and dot");
            while let Ok(event) = eval.recv().await {
                tracing::info!(?event, "recieved an event");
            }
        }
        .in_current_span()
    });

    cx.render(rsx!(div { id: id }))
}
