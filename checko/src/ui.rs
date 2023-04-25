use std::{cmp::Reverse, time::Duration};

use axum::{extract::WebSocketUpgrade, response::Html, routing::get, Router};
use dioxus::prelude::*;
use itertools::Itertools;
use tracing::info;

use crate::{
    batch::{Batch, Group, GroupStage},
    test_runner::{TestResultType, TestRunData},
};

#[derive(Clone)]
pub struct AppState {
    batch: Batch,
}

static APP_STATE: once_cell::sync::OnceCell<AppState> = once_cell::sync::OnceCell::new();
impl AppState {
    pub fn set_global(state: Self) {
        APP_STATE.set(state);
    }
    pub fn global() -> Option<Self> {
        APP_STATE.get().cloned()
    }
}

pub async fn start_web_ui(batch: Batch) -> color_eyre::Result<()> {
    AppState::set_global(AppState { batch });

    let addr: std::net::SocketAddr = ([0, 0, 0, 0], 3030).into();

    let view = dioxus_liveview::LiveViewPool::new();

    let app = Router::new()
        // The root route contains the glue code to connect to the WebSocket
        .route(
            "/",
            get(move || async move {
                Html(format!(
                    r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Checko</title>
                    <script src="https://cdn.tailwindcss.com"></script>
                </head>
                <body class="bg-slate-900 text-white"> <div id="main"></div> </body>
                {glue}
                </html>
                "#,
                    // Create the glue code to connect to the WebSocket on the "/ws" route
                    glue = dioxus_liveview::interpreter_glue(&format!("ws://{addr}/ws"))
                ))
            }),
        )
        // The WebSocket route is what Dioxus uses to communicate with the browser
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| async move {
                    // When the WebSocket is upgraded, launch the LiveView with the app component
                    _ = view.launch(dioxus_liveview::axum_socket(socket), app).await;
                })
            }),
        );

    info!("Live results at http://{addr}");

    axum::Server::bind(&addr.to_string().parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Color {
    Green, //#15803d
    Orange,
    Red,
    Blue,
}

impl Color {
    fn hex(self) -> &'static str {
        match self {
            Color::Green => "#15803d",
            Color::Orange => "#f97316",
            Color::Red => "#ef4444",
            // TODO
            Color::Blue => "blue",
        }
    }
    fn tw(self) -> &'static str {
        match self {
            Color::Green => "green-700",
            Color::Orange => "orange-500",
            Color::Red => "red-500",
            Color::Blue => "blue-700",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Row {
    pub name: String,
    pub status: String,
    pub colors: Vec<Color>,
    pub details: Result<Vec<(String, Vec<(usize, (String, Color))>)>, String>,
}

impl Row {
    pub fn from_groups<'a>(
        gs: impl IntoIterator<Item = &'a Group>,
        dedup: bool,
        respect_shown: bool,
    ) -> Vec<Self> {
        gs.into_iter()
            .sorted_by_key(|g| {
                let (num_correct, time) = match &*g.stage.read().unwrap() {
                    GroupStage::Initial => (0, Duration::ZERO),
                    GroupStage::TestsRun { results, .. } => {
                        let num_correct: usize = results
                            .iter()
                            .map(|r| match &r.data {
                                TestRunData::CompileError(_) => 0,
                                TestRunData::Sections(ss) => {
                                    1 + ss
                                        .iter()
                                        .flat_map(|s| {
                                            s.programs.iter().filter(|p| p.result.is_correct())
                                        })
                                        .count()
                                }
                            })
                            .sum();
                        let time: Duration = results
                            .iter()
                            .map(|r| match &r.data {
                                TestRunData::CompileError(_) => Duration::ZERO,
                                TestRunData::Sections(ss) => ss
                                    .iter()
                                    .flat_map(|s| {
                                        s.programs
                                            .iter()
                                            .filter(|p| p.result.is_correct())
                                            .map(|p| p.time)
                                    })
                                    .sum(),
                            })
                            .sum();
                        (1 + num_correct, time)
                    }
                };
                (Reverse(num_correct), time)
            })
            .map(|g| Row::from_group(g, dedup, respect_shown))
            .collect()
    }
    pub fn from_group(g: &Group, dedup: bool, respect_shown: bool) -> Self {
        let details = match &*g.stage.read().unwrap() {
            GroupStage::Initial => Err("".to_owned()),
            GroupStage::TestsRun { results, .. } => match results {
                Ok(r) => match &r.data {
                    TestRunData::CompileError(err) => Err(err.to_string()),
                    TestRunData::Sections(sections) => Ok(sections
                        .iter()
                        .map(|s| {
                            let ps = s.programs.iter().map(|p| {
                                let show = if respect_shown { p.shown } else { true };
                                match &p.result {
                                    TestResultType::CorrectTerminated => {
                                        ("Correct".to_string(), Color::Green)
                                    }
                                    TestResultType::CorrectNonTerminated { .. } => {
                                        ("Correct*".to_string(), Color::Green)
                                    }
                                    TestResultType::Mismatch { reason } => (
                                        if show {
                                            format!("Mismatch: {reason}")
                                        } else {
                                            "Mismatch".to_string()
                                        },
                                        Color::Orange,
                                    ),
                                    TestResultType::TimeOut => {
                                        ("Time out".to_string(), Color::Blue)
                                    }
                                    TestResultType::Error { description } => (
                                        if show {
                                            description.to_string()
                                        } else {
                                            "Error".to_string()
                                        },
                                        Color::Red,
                                    ),
                                }
                            });

                            (s.analysis.to_string(), ps.collect_vec())
                        })
                        .collect_vec()),
                },
                Err(err) => Err(err.to_string()),
            },
        };

        let colors = details
            .iter()
            .flat_map(|d| d.iter().flat_map(|(_, ps)| ps.iter().map(|(_, c)| *c)))
            .collect_vec();
        let details = details.map(|rows| {
            rows.into_iter()
                .map(|(analysis, ps)| {
                    (
                        analysis,
                        if dedup {
                            ps.into_iter().dedup_with_count().collect()
                        } else {
                            ps.into_iter().map(|c| (1, c)).collect()
                        },
                    )
                })
                .collect()
        });

        let status = match &*g.stage.read().unwrap() {
            GroupStage::Initial => "Not run".to_string(),
            GroupStage::TestsRun { results, .. } => match results {
                Ok(r) => match &r.data {
                    TestRunData::CompileError(_) => "Compile error".to_string(),
                    TestRunData::Sections(s) => {
                        let result = s
                            .iter()
                            .map(|x| {
                                let correct =
                                    x.programs.iter().filter(|p| p.result.is_correct()).count();
                                format!("{correct:>5}/{}", x.programs.len())
                            })
                            .join(" | ");
                        format!("{result}")
                    }
                },
                Err(_) => "Errored".to_string(),
            },
        };

        Row {
            name: g.config.name.clone(),
            status,
            colors,
            details,
        }
    }
}

#[inline_props]
pub fn GroupRow(cx: Scope, row: Row, no_details: Option<bool>, dedup: bool, open: bool) -> Element {
    let no_details = no_details.unwrap_or_default();
    cx.render(rsx!(
        details {
            class: "group",
            open: if *open { Some("open") } else { None },
            summary {
                class: "flex space-x-2 px-2 sticky top-0 group-open:bg-slate-700 transition-all group-open:py-2",
                div { class: "w-24 font-bold font-mono", "{row.name}" }
                div { class: "w-64", "{row.status}" }
                div {
                    class: "flex-1 flex",
                    for color in &row.colors { div { class: "flex-1 bg-{color.tw()}" } }
                }
            }
            if !no_details {
                rsx!(div {
                    class: "p-2",
                    match &row.details {
                        Ok(sec) => {
                            rsx!(sec.iter().map(|(a, ps)| {
                                rsx!(div {
                                    class: "space-y-0.5",
                                    p { "{a}" }
                                    div {
                                        class: "divide-y divide-slate-600",
                                        ps.iter().map(|(c, (p, color))| {
                                            rsx!(div {
                                                if *dedup { rsx!(p { class: "text-xs text-slate-500 italic p-1", "repeated {c} times" }) }
                                                pre { class: "text-xs border-l-4 border-{color.tw()} pl-2 pb-1", "{p}" }
                                            })
                                        })
                                    }
                                })
                            }))
                        }
                        Err(e) => rsx!(pre { class: "text-xs p-1 border-l border-red-500 text-red-50", "{e}" })
                    }
                })
            }
        }
    ))
}

#[inline_props]
pub fn AdminView(cx: Scope, rows: Vec<Row>) -> Element {
    cx.render(rsx!(
        div {
            h1 { class: "flex items-center space-x-2 p-4 text-2xl font-thin",
                span { "🤖" } span { class: "italic", " Checko" }
            }
            div {
                rows.iter().map(|row| rsx!(
                    GroupRow { key: "{row.name}", row: row.clone(), dedup: true, open: false }
                ))
            }
        }
    ))
}

#[inline_props]
pub fn PublicView(cx: Scope, rows: Vec<Row>) -> Element {
    cx.render(rsx!(
        div {
            h1 { class: "flex items-center space-x-2 p-4 text-2xl font-thin",
                span { "🤖" } span { class: "italic", " Checko" }
            }
            div {
                rows.iter().map(|row| rsx!(
                    GroupRow { key: "{row.name}", row: row.clone(), no_details: true, dedup: true, open: false }
                ))
            }
        }
    ))
}

const WIDTH: usize = 15;
const HEIGHT: usize = 20;

#[inline_props]
pub fn SvgRow(cx: Scope, row: Row, no_details: Option<bool>, dedup: bool, open: bool) -> Element {
    cx.render(rsx!(
        g {
            for (x, color) in row.colors.iter().enumerate() {
                rect { fill: "{color.hex()}", x: "{x * WIDTH}", width: "{WIDTH}", height: "{HEIGHT}", }
            }
        }
        text {
            "dominant-baseline": "middle",
            y: "{HEIGHT/2}",
            "{row.name} {row.status}"
        }
    ))
}

#[inline_props]
pub fn SvgTable(cx: Scope, rows: Vec<Row>) -> Element {
    let w = rows
        .iter()
        .map(|r| r.colors.len())
        .max()
        .unwrap_or_default();
    let h = rows.len();

    cx.render(rsx!(
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            style: "font-family: monospace; font-size: {HEIGHT as f32 * 0.5}px",
            width: "{w*WIDTH}",
            height: "{h*HEIGHT}",
            "view-box": "0 0 {w*WIDTH} {h*HEIGHT}",
            rows.iter().enumerate().map(|(y, row)| rsx!(
                g {
                    transform: "translate(0, {y * HEIGHT})",
                    SvgRow { key: "{row.name}", row: row.clone(), no_details: true, dedup: true, open: false }
                }
            ))
        }
    ))
}

fn app(cx: Scope) -> Element {
    let rows = use_state(cx, || vec![]);

    use_coroutine(cx, |_rx: dioxus::prelude::UnboundedReceiver<()>| {
        let jobs = rows.to_owned();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                jobs.set(Row::from_groups(
                    AppState::global().unwrap().batch.groups.values(),
                    true,
                    false,
                ));
            }
        }
    });

    cx.render(rsx!(AdminView {
        rows: rows.to_vec()
    }))
}
