#![allow(non_snake_case)]

use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{extract::WebSocketUpgrade, response::Html, routing::get, Router};
use ce_core::rand::{self, SeedableRng};
use ce_shell::{Analysis, Input};
use dioxus::prelude::*;
use driver::{
    ansi::{self, Color, Span},
    Driver, Hub, JobId, JobState,
};
use futures_util::StreamExt;
use itertools::Itertools;
use tracing::Instrument;
use tracing_subscriber::prelude::*;

type Job = driver::Job<()>;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    tracing_subscriber::Registry::default()
        .with(tracing_error::ErrorLayer::default())
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .without_time(),
        )
        .with(tracing_subscriber::filter::FilterFn::new(|m| {
            !m.target().contains("hyper")
        }))
        .init();

    run().await
}

async fn run() -> color_eyre::Result<()> {
    let hub = Hub::default();
    let driver = Driver::new_from_path(hub.clone(), "./run.toml")?;
    driver.start_recompile();

    driver.spawn_watcher()?;

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

    let view = dioxus_liveview::LiveViewPool::new();

    // let app = Router::new()
    //     // The root route contains the glue code to connect to the WebSocket
    //     .route(
    //         "/",
    //         get(move || async move {
    //             Html(
    //                 include_str!("./index.html")
    //                     .replace(
    //                         "%head%",
    //                         &format!("<style>{}</style>", include_str!("../public/tailwind.css")),
    //                     )
    //                     .replace(
    //                         "%glue%",
    //                         &dioxus_liveview::interpreter_glue(&format!("ws://{addr}/ws")),
    //                     ),
    //             )
    //         }),
    //     )
    //     // The WebSocket route is what Dioxus uses to communicate with the browser
    //     .route(
    //         "/ws",
    //         get(move |ws: WebSocketUpgrade| async move {
    //             ws.on_upgrade(move |socket| async move {
    //                 // When the WebSocket is upgraded, launch the LiveView with the app component
    //                 _ = view
    //                     .launch_with_props(
    //                         dioxus_liveview::axum_socket(socket),
    //                         App,
    //                         AppProps { hub, driver },
    //                     )
    //                     .await;
    //             })
    //         }),
    //     );

    // println!("Listening on http://{addr}");

    // axum::Server::bind(&addr.to_string().parse().unwrap())
    //     .serve(app.into_make_service())
    //     .await
    //     .unwrap();

    Ok(())
}

fn use_analysis(cx: &ScopeState) -> Analysis {
    *use_shared_state(cx).unwrap().read()
}

type SharedHubDriver = (Hub<()>, Driver);

fn use_hub(cx: &ScopeState) -> Hub<()> {
    use_shared_state::<SharedHubDriver>(cx)
        .unwrap()
        .read()
        .0
        .clone()
}
fn use_driver(cx: &ScopeState) -> Driver {
    use_shared_state::<SharedHubDriver>(cx)
        .unwrap()
        .read()
        .1
        .clone()
}

fn use_updated<T, S>(cx: &ScopeState, input: T, f: impl Fn(&T) -> S + 'static) -> &S
where
    T: Clone + PartialEq + Send + Sync + 'static,
    S: Clone + PartialEq + 'static,
{
    let value = use_state(cx, || f(&input));
    let active_input = use_memo(cx, (), {
        let input = input.clone();
        move |_| Arc::new(Mutex::new(input))
    });
    use_effect(cx, (&input,), |(input,)| {
        to_owned![value, active_input];
        *active_input.lock().unwrap() = input.clone();
        async move {
            loop {
                if input != *active_input.lock().unwrap() {
                    break;
                }
                let new = f(&input);
                if *value != new {
                    value.set(new);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    });
    value
}

fn use_jobs(cx: &ScopeState) -> &[Job] {
    let hub = use_hub(cx);
    use_updated(cx, hub, |hub| hub.jobs())
}

fn use_latest_successfull_compile(cx: &ScopeState) -> Option<Job> {
    let driver = use_driver(cx);
    use_updated(cx, driver, |driver| driver.latest_successfull_compile()).clone()
}

#[derive(Props, PartialEq)]
struct AppProps {
    hub: Hub<()>,
    driver: Driver,
}

fn App(cx: Scope<AppProps>) -> Element {
    use_shared_state_provider(cx, || Analysis::Sign);
    use_shared_state_provider::<SharedHubDriver>(cx, || {
        (cx.props.hub.clone(), cx.props.driver.clone())
    });

    cx.render(rsx! {
        div {
            class: "grid h-screen grid-rows-[auto_1fr_auto_auto]",
            Nav {}
            ViewEnv {}
            StatusBar {}
        }
    })
}

fn Nav(cx: Scope) -> Element {
    use ce_core::dioxus_heroicons::{mini::Shape, Icon};
    let analysis = use_shared_state(cx).unwrap();

    cx.render(rsx!(
        nav {
            class: "flex items-center bg-slate-900 px-2 text-slate-200",
            a {
                href: "/",
                class: "flex items-center space-x-2 p-2 pr-0 text-2xl font-thin italic",
                div {
                    class: "relative",
                    Icon {
                        class: "absolute inset-0 left-0.5 top-0.5 w-6 animate-pulse text-teal-500/50",
                        icon: Shape::CommandLine,
                    }
                    Icon {
                        class: "relative w-6",
                        icon: Shape::CommandLine,
                    }
                }
                span { "Inspectify"}
            }
            select {
                class: "bg-transparent py-0 border-none ml-4 text-right",
                oninput: |evt| {
                    *analysis.write() = Analysis::from_str(&evt.data.value).unwrap();
                },
                for o in Analysis::options() {
                    option {
                        selected: o == &*analysis.read(),
                        "{o}"
                    }
                }
            }
            div { class: "flex-1" }
            a {
                href: "/",
                class: "flex items-center space-x-1 p-2 text-sm font-semibold text-slate-300 transition hover:text-white",
                span { "Analysis" }
                Icon {
                    class: "w-4",
                    icon: Shape::PlayCircle,
                }
            }
            a {
                href: "/guide.html",
                class: "flex items-center space-x-1 p-2 text-sm font-semibold text-slate-300 transition hover:text-white",
                span { "Guide" }
                Icon {
                    class: "w-4",
                    icon: Shape::QuestionMarkCircle,
                }
            }
        }
    ))
}

fn StatusBar(cx: Scope) -> Element {
    use ce_core::dioxus_heroicons::{mini::Shape, Icon};

    let hub = use_hub(cx);

    use_shared_state_provider(cx, || None as Option<JobId>);
    let selected_job_id = use_shared_state::<Option<JobId>>(cx).unwrap();

    let selected_job = selected_job_id.read().and_then(|id| hub.get_job(id));

    let show = use_state(cx, || true);

    let active_jobs = {
        let jobs = use_jobs(cx);
        let mut queued = 0;
        let mut running = 0;
        let mut succeeded = 0;
        let mut canceled = 0;
        let mut failed = 0;
        let mut warning = 0;

        for job in jobs {
            match job.state() {
                JobState::Queued => queued += 1,
                JobState::Running => running += 1,
                JobState::Succeeded => succeeded += 1,
                JobState::Canceled => canceled += 1,
                JobState::Failed => failed += 1,
                JobState::Warning => warning += 1,
            }
        }

        match (queued, running, succeeded, failed, warning) {
            (0, 0, _, 0, 0) => cx.render(rsx!("No active jobs")),
            _ => {
                let status = [
                    ("queued", queued),
                    ("running", running),
                    ("succeeded", succeeded),
                    ("canceled", canceled),
                    ("failed", failed),
                    ("warning", warning),
                ]
                .into_iter()
                .filter(|(_, n)| *n > 0)
                .map(|(state, c)| cx.render(rsx!("{c} {state}")))
                .intersperse(cx.render(rsx!(", ")));
                cx.render(rsx!(b { "Jobs: " } i { status }))
            }
        }
    };

    let version = option_env!("GITHUB_REF_NAME").unwrap_or(env!("CARGO_PKG_VERSION"));

    cx.render(rsx!(div {
        class: "grid grid-flow-row",
        if **show {
            rsx!(div {
                class: "bg-slate-950 border-t grid grid-cols-[20ch_1fr] grid-rows-[35vh]",
                JobList {}
                if let Some(selected_job) = selected_job {
                    cx.render(rsx!(JobView {
                        job: selected_job,
                    }))
                }
            })
        }
        div {
            class: "bg-slate-900 text-sm flex border-t space-x-1 items-center",
            button {
                class: "bg-slate-900 h-full px-2 text-xs flex space-x-0.5 items-center hover:bg-slate-400/10 active:bg-slate-400/5 transition",
                onclick: move |_| show.set(!show),
                Icon {
                    class: if **show { "transition rotate-0" } else { "transition rotate-180" },
                    size: 10,
                    icon: Shape::ChevronDoubleUp,
                }
                span { active_jobs }
            }
            div { class: "flex-1" }
            div {
                class: "text-slate-400 text-xs",
                "v{version}"
            }
            div {
                class: "place-self-end bg-green-600 p-1",
                Icon {
                    size: 13,
                    icon: Shape::Link,
                }
            }
        }
    }))
}

#[inline_props]
fn JobView(cx: Scope, job: Job) -> Element {
    let stdout_and_stderr = use_updated(cx, job.clone(), |job| job.stdout_and_stderr());

    let spans = ansi::parse_ansi(stdout_and_stderr);

    cx.render(rsx!(div {
        class: "text-xs border-l relative self-stretch bg-slate-900",
        div {
            class: "absolute inset-0 overflow-auto",
            pre {
                class: "p-3 [overflow-anchor:none]",
                code {
                    for span in spans.iter().filter(|span| !span.text.is_empty()) {
                        ViewSpan { span: *span }
                    }
                }
            }
            div { class: "[overflow-anchor:auto]" }
        }
    }))
}

#[inline_props]
fn ViewSpan<'a>(cx: Scope<'a>, span: Span<'a>) -> Element<'a> {
    let fg_class = match span.fg {
        Some(c) => match c {
            Color::Black => "text-black",
            Color::Red => "text-red-500",
            Color::Green => "text-green-500",
            Color::Yellow => "text-yellow-500",
            Color::Blue => "text-blue-500",
            Color::Magenta => "text-magenta-500",
            Color::Cyan => "text-cyan-500",
            Color::White => "text-white",
            Color::Default => "",
            Color::BrightBlack => "text-black",
            Color::BrightRed => "text-red-300",
            Color::BrightGreen => "text-green-300",
            Color::BrightYellow => "text-yellow-300",
            Color::BrightBlue => "text-blue-300",
            Color::BrightMagenta => "text-magenta-300",
            Color::BrightCyan => "text-cyan-300",
            Color::BrightWhite => "text-white",
        },
        None => "",
    };
    let bg_class = match span.bg {
        Some(c) => match c {
            Color::Black => "bg-black",
            Color::Red => "bg-red-500",
            Color::Green => "bg-green-500",
            Color::Yellow => "bg-yellow-500",
            Color::Blue => "bg-blue-500",
            Color::Magenta => "bg-magenta-500",
            Color::Cyan => "bg-cyan-500",
            Color::White => "bg-white",
            Color::Default => "",
            Color::BrightBlack
            | Color::BrightRed
            | Color::BrightGreen
            | Color::BrightYellow
            | Color::BrightBlue
            | Color::BrightMagenta
            | Color::BrightCyan
            | Color::BrightWhite => {
                tracing::debug!(color=?c, "unhandled span");
                ""
            }
        },
        None => "",
    };

    if fg_class.is_empty() && bg_class.is_empty() {
        cx.render(rsx!("{span.text}"))
    } else {
        cx.render(rsx!(span { class: "{fg_class} {bg_class}", "{span.text}" }))
    }
}

fn JobList(cx: Scope) -> Element {
    let jobs = use_jobs(cx);
    let selected_job = use_shared_state::<Option<JobId>>(cx).unwrap();

    let remove_duplicates = false;
    let mut seen_kinds = HashSet::new();

    let jobs = jobs
        .iter()
        .cloned()
        .rev()
        .filter(|j| {
            if remove_duplicates {
                seen_kinds.insert(j.kind())
            } else {
                true
            }
        })
        .collect_vec();

    use_effect(cx, (&jobs,), |(jobs,)| {
        to_owned![selected_job];
        async move {
            *selected_job.write() = jobs.first().map(|j| j.id());
        }
    });

    cx.render(rsx!(div {
        class: "relative text-sm",
        div {
            class: "absolute inset-0 overflow-auto grid items-start",
            div {
                class: "grid grid-cols-[1fr_1fr]",
                div { class: "py-1 px-2 font-bold text-center sticky top-0 bg-slate-950", "Job" }
                div { class: "py-1 px-2 font-bold text-center sticky top-0 bg-slate-950", "State" }
                for job in jobs {
                    JobRow {
                        key: "{job.id():?}",
                        job: job.clone(),
                        selected: Some(&job.id()) == selected_job.read().as_ref(),
                        onclick: move |_| *selected_job.write() = Some(job.id()),
                    }
                }
            }
        }
    }))
}

#[derive(Props)]
struct JobRowProps<'a> {
    job: Job,
    selected: bool,
    onclick: EventHandler<'a>,
}
fn JobRow<'a>(cx: Scope<'a, JobRowProps<'a>>) -> Element<'a> {
    use ce_core::dioxus_heroicons::{mini::Shape, Icon};

    let job = &cx.props.job;

    let class = if cx.props.selected {
        "bg-slate-700"
    } else {
        "group-hover:bg-slate-800"
    };

    let (icon, icon_class) = match job.state() {
        JobState::Queued => (Shape::EllipsisHorizontal, "animate-pulse"),
        JobState::Running => (Shape::ArrowPath, "animate-spin text-slate-400"),
        JobState::Succeeded => (Shape::Check, "text-green-300"),
        JobState::Canceled => (Shape::NoSymbol, "text-slate-400"),
        JobState::Failed => (Shape::Fire, "text-red-300"),
        JobState::Warning => (Shape::ExclamationTriangle, "text-yellow-300"),
    };

    cx.render(rsx!(button {
        class: "contents group text-left",
        onclick: move |_| cx.props.onclick.call(()),
        div { class: "pl-2 pr-1 py-0.5 transition {class}", "{job.kind()}" }
        div {
            class: "px-1 py-0.5 flex justify-center items-center transition {class}",
            title: "{job.state()}",
            Icon {
                class: icon_class,
                icon: icon,
                size: 16,
            }
        }
    }))
}

fn ViewEnv(cx: Scope) -> Element {
    let analysis = use_analysis(cx);

    let driver = use_driver(cx);
    let latest_successfull_compile = use_latest_successfull_compile(cx);

    let input = use_state(cx, || {
        analysis.gen_input(&mut rand::rngs::SmallRng::from_entropy())
    });
    let real_output = use_state(cx, || None);

    let set_input = use_coroutine::<Input, _, _>(cx, |mut rx| {
        to_owned![input, real_output];
        async move {
            while let Some(new) = rx.next().await {
                input.set(new);
                real_output.set(None);
            }
        }
    });

    use_effect(cx, (&analysis,), |(analysis,)| {
        to_owned![set_input];
        async move {
            set_input.send(analysis.gen_input(&mut rand::rngs::SmallRng::from_entropy()));
        }
    });

    let last_job = use_state::<Option<Job>>(cx, || None);

    use_effect(
        cx,
        (input.get(), &latest_successfull_compile),
        |(input, _job)| {
            to_owned![driver, real_output, last_job];
            real_output.set(None);
            let analysis = input.analysis();
            async move {
                if let Some(j) = &*last_job {
                    j.kill();
                }

                let job = driver.exec_job(&input);
                last_job.set(Some(job.clone()));
                job.wait().await;
                let stdout = job.stdout();
                if stdout.is_empty() {
                    return;
                }
                let output = input
                    .analysis()
                    .parse_output(&stdout)
                    .expect("failed to parse output");
                real_output.set(Some(output));
            }
            .instrument(tracing::info_span!("running analysis", ?analysis))
        },
    );

    analysis.render(cx, input, set_input.clone(), real_output)
}
