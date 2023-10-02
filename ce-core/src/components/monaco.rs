use dioxus::prelude::*;
use tracing::Instrument;

#[derive(Props)]
pub struct MonacoEditorProps<'a> {
    pub value: String,
    pub on_change: EventHandler<'a, String>,
}

#[tracing::instrument(skip_all)]
pub fn MonacoEditor<'a>(cx: Scope<'a, MonacoEditorProps>) -> Element<'a> {
    let create_eval = use_eval(cx);

    let id = cx.raw_text(format_args!("monaco-editor-{}", cx.scope_id().0));

    let last_change = use_state(cx, || None as Option<String>);

    use_effect(cx, (&id.to_string(), &cx.props.value), |(id, value)| {
        to_owned![last_change];
        let eval = create_eval(
            &include_str!("./monaco.js")
                .replace("%id%", &id)
                .replace("\"%value%\"", &serde_json::to_string(&value).unwrap()),
        )
        .unwrap();

        async move {
            tracing::info!("sending id and value");
            while let Ok(new) = eval.recv().await {
                tracing::info!("recieved an event");
                last_change.set(Some(new.as_str().unwrap().to_string()));
            }
        }
        .in_current_span()
    });

    if let Some(change) = last_change.get() {
        tracing::info!("taking the last change!");
        last_change.set(None);
        if change != &cx.props.value {
            cx.props.on_change.call(change.clone());
        }
    }

    cx.render(rsx!(div { id: id }))
}
