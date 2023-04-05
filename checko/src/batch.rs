use std::{
    collections::HashMap,
    hash::Hasher,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use tracing::{error, info, span, warn, Instrument, Level};
use xshell::{cmd, Shell};

use crate::{
    collect_programs,
    config::{CanonicalProgramsConfig, GroupConfig},
    docker::DockerImage,
    fmt::{CompetitionMarkdown, IndividualMarkdown},
    group_env::{set_checko_git_account, GroupEnv},
    read_groups, retry,
    test_runner::{TestRunData, TestRunInput, TestRunResults},
    ui,
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Batch {
    #[serde(skip)]
    pub path: Option<PathBuf>,
    pub programs: Arc<CanonicalProgramsConfig>,
    pub groups: IndexMap<String, Group>,
}

impl Batch {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let path = path.canonicalize().with_context(|| {
            format!("failed to canonicalize batch path at '{}'", path.display())
        })?;
        let mut batch: Self = serde_json::from_str(
            &std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read batch at '{}'", path.display()))?,
        )?;

        batch.path = Some(path);

        Ok(batch)
    }
    pub async fn write_to_disk(&self) -> Result<()> {
        if let Some(path) = &self.path {
            Ok(tokio::fs::write(path, serde_json::to_string(self)?).await?)
        } else {
            bail!("writing to disk failed. batch did not have a path specified")
        }
    }
    pub fn write_to_disk_sync(&self) -> Result<()> {
        if let Some(path) = &self.path {
            Ok(std::fs::write(path, serde_json::to_string(self)?)?)
        } else {
            bail!("writing to disk failed. batch did not have a path specified")
        }
    }
    fn reset_all(&self) {
        for g in self.groups.keys() {
            self.reset(g);
        }
    }

    fn reset(&self, g: &str) {
        *self.groups[g].stage.write().unwrap() = Default::default();
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Group {
    pub config: GroupConfig,
    pub stage: Arc<RwLock<GroupStage>>,
    #[serde(default, skip)]
    pub status: Arc<RwLock<GroupStatus>>,
}
impl Group {
    async fn work(&self, image: &DockerImage, programs: Arc<CanonicalProgramsConfig>) {
        let stage = self.stage.read().unwrap().clone();
        match stage {
            GroupStage::Initial => {
                let env = GroupEnv::new(&self.config).unwrap();

                // TODO: Replace this with a try-block when they are stable
                let result: Result<TestRunResults> = (|| async {
                    let sh = env
                        .shell_in_default_branch()
                        .wrap_err("getting shell in default branch")?;
                    let cwd = sh.current_dir();
                    drop(sh);
                    let output =
                        TestRunInput::run_in_docker(image, &cwd, programs.as_ref().clone()).await?;
                    match &output.data {
                        TestRunData::CompileError(_) => {
                            warn!("failed to compile. compile error saved")
                        }
                        TestRunData::Sections(_) => {}
                    }
                    Ok(output)
                })()
                .await;

                info!("done!");
                let results = result.map_err(|err| format!("{err:?}"));
                *self.stage.write().unwrap() = GroupStage::TestsRun {
                    results,
                    pushed_results: false,
                };
            }
            GroupStage::TestsRun { .. } => {}
        }
    }
}

#[derive(Debug, Default, Hash, Clone, Serialize, Deserialize)]
pub enum GroupStage {
    #[default]
    Initial,
    TestsRun {
        results: Result<TestRunResults, String>,
        pushed_results: bool,
    },
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum GroupStatus {
    #[default]
    Idle,
    Cloning,
    RunningTests,
    TestsRan,
}

#[derive(Debug, clap::Subcommand)]
pub enum BatchCli {
    Init {
        /// The configs file specifying the programs to run in the competition.
        #[clap(long, short)]
        programs: Vec<PathBuf>,
        /// The configs file specifying the groups which are part of the competition.
        #[clap(long, short)]
        groups: PathBuf,
        /// The name of the batch. Defaults to current time.
        #[clap(short, long, default_value_t = default_batch_name())]
        name: String,
    },
    Work {
        /// When set will build the docker image from the source
        #[clap(long, short, default_value_t = false)]
        local_docker: bool,
        /// Number of concurrent projects being evaluated.
        #[clap(long, short, default_value_t = 2)]
        concurrent: usize,
        /// Keep the server running even after work is done.
        #[clap(long, short, default_value_t = false)]
        keep_alive: bool,
        /// The path to the batch JSON file.
        batch_path: PathBuf,
    },
    Reset {
        /// The path to the batch JSON file.
        batch_path: PathBuf,
        /// Reset *all* groups. Use with caution!
        #[clap(long, default_value_t = false)]
        all: bool,
        /// The groups to reset.
        groups: Vec<String>,
    },
    Status {
        /// The path to the batch JSON file.
        batch_path: PathBuf,
    },
    Static {
        /// The path to the batch JSON file.
        batch_path: PathBuf,
        /// The path to write the static site.
        output: PathBuf,
    },
    Competition {
        /// The path to the batch JSON file.
        batch_path: PathBuf,
        /// The path to write the Markdown.
        output: PathBuf,
    },
    Publish {
        /// The path to the batch JSON file.
        batch_path: PathBuf,
    },
}

fn default_batch_name() -> String {
    let now = chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now());
    format!("batch-{}", now.format("%+"))
}

impl BatchCli {
    pub async fn run(self) -> Result<()> {
        match self {
            BatchCli::Init {
                programs,
                groups,
                name,
            } => {
                let programs = collect_programs(programs)?;
                let groups = read_groups(groups)?;

                let write_path = PathBuf::from(format!("{name}.json"));
                let batch = Batch {
                    path: Some(write_path.clone()),
                    programs: Arc::new(programs.canonicalize()?),
                    groups: groups
                        .groups
                        .into_iter()
                        .map(|config| {
                            let name = config.name.clone();
                            let g = Group {
                                config,
                                ..Default::default()
                            };
                            (name, g)
                        })
                        .collect(),
                };

                batch.write_to_disk().await?;

                info!("batch written to {}", write_path.display());
                println!("{}", write_path.display());
            }
            BatchCli::Work {
                local_docker,
                concurrent,
                keep_alive,
                batch_path,
            } => {
                let batch = Batch::from_path(&batch_path)?;

                // NOTE: Check of docker daemon is running
                {
                    let sh = Shell::new()?;
                    cmd!(sh, "docker ps")
                        .quiet()
                        .ignore_stdout()
                        .run()
                        .wrap_err("docker does not seem to be running")?;
                }

                let image = if local_docker {
                    DockerImage::build_in_tree().await?
                } else {
                    DockerImage::build().await?
                };

                let spinner = indicatif::ProgressBar::new(batch.groups.len() as _);
                spinner.set_style(
                    indicatif::ProgressStyle::with_template(
                        "[{elapsed_precise}] {bar:120.cyan/blue} {pos:>7}/{len:7} {msg}",
                    )
                    .unwrap(),
                );

                let task_permits = Arc::new(tokio::sync::Semaphore::new(concurrent));
                let mut tasks = JoinSet::new();

                for g in batch.groups.values().cloned() {
                    let batch = batch.clone();
                    let programs = Arc::clone(&batch.programs);
                    let image = image.clone();
                    let task_permits = task_permits.clone();

                    tasks.spawn(async move {
                        let _permit = task_permits.acquire().await.unwrap();
                        g.work(&image, programs)
                            .instrument(span!(
                                Level::INFO,
                                "Group",
                                name = g.config.name,
                                // nr = format!("{}/{num_groups}", g_idx + 1)
                            ))
                            .await;
                        batch.write_to_disk().await.unwrap();
                    });
                }

                if keep_alive {
                    tasks.spawn(async { ui::start_web_ui(batch).await.unwrap() });
                } else {
                    tokio::spawn(ui::start_web_ui(batch));
                }

                while tasks.join_next().await.is_some() {
                    spinner.inc(1);
                }

                spinner.finish();
            }
            BatchCli::Reset {
                batch_path,
                all,
                groups,
            } => {
                let batch = Batch::from_path(batch_path)?;
                if all {
                    batch.reset_all();
                } else {
                    for g in &groups {
                        batch.reset(g);
                    }
                }
                batch.write_to_disk().await?;
            }
            BatchCli::Status { batch_path } => {
                let batch_path = batch_path.canonicalize()?;
                let batch: Batch =
                    serde_json::from_str(&std::fs::read_to_string(&batch_path).with_context(
                        || format!("failed to read batch at '{}'", batch_path.display()),
                    )?)?;

                ui::start_web_ui(batch).await?;
            }
            BatchCli::Static { batch_path, output } => {
                use dioxus::prelude::*;

                let batch = Batch::from_path(batch_path)?;

                let hash = {
                    use std::hash::Hash;
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    for (_, g) in &batch.groups {
                        g.stage.read().unwrap().hash(&mut hasher);
                    }
                    hasher.finish()
                };
                let output = output.join(format!("batch-{hash}"));

                let rows = ui::Row::from_groups(batch.groups.values(), true, false);

                let admin_rows = rows.clone();
                let admin_hash = {
                    use std::hash::Hash;
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    admin_rows.hash(&mut hasher);
                    hasher.finish()
                };
                let admin = dioxus_ssr::render_lazy(rsx!(ui::AdminView { rows: admin_rows }));
                let public_rows = rows.clone();
                let public = dioxus_ssr::render_lazy(rsx!(ui::PublicView { rows: public_rows }));

                std::fs::create_dir_all(&output)?;
                std::fs::write(
                    output.join(format!("admin-{admin_hash}.html")),
                    layout(&admin),
                )?;
                std::fs::write(output.join("index.html"), layout(&public))?;

                let svg = dioxus_ssr::render_lazy(rsx!(ui::SvgTable { rows: rows }));
                std::fs::write(output.join("graph.svg"), svg)?;

                let group_files: HashMap<_, _> = batch
                    .groups
                    .values()
                    .map(|g| {
                        let row = ui::Row::from_group(g, false, true);
                        let hash = {
                            use std::hash::Hash;
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            row.hash(&mut hasher);
                            hasher.finish()
                        };
                        let html = dioxus_ssr::render_lazy(rsx!(ui::GroupRow {
                            row: row,
                            dedup: false,
                            open: true,
                        }));
                        let dest = format!("{}-{hash}.html", g.config.name);
                        std::fs::write(output.join(&dest), layout(&html)).unwrap();
                        (g.config.name.clone(), dest)
                    })
                    .collect();

                std::fs::write(
                    output.join("manifest.toml"),
                    toml::to_string_pretty(&group_files)?,
                )?;
            }
            BatchCli::Competition { batch_path, output } => {
                let batch = Batch::from_path(batch_path)?;

                let mut input = CompetitionMarkdown::default();

                for g in batch.groups.values() {
                    let span = span!(Level::INFO, "Group", name = g.config.name);
                    let _enter = span.enter();
                    match &*g.stage.read().unwrap() {
                        GroupStage::Initial => {
                            warn!("still haven't run");
                        }
                        GroupStage::TestsRun {
                            results,
                            pushed_results: _,
                        } => match results {
                            Ok(r) => match &r.data {
                                TestRunData::CompileError(_) => {
                                    error!("did not have a latest run (they failed to compile)")
                                }
                                TestRunData::Sections(sections) => {
                                    for sec in sections {
                                        input
                                            .sections
                                            .entry(sec.analysis)
                                            .or_default()
                                            .entry(g.config.name.clone())
                                            .or_insert(sec.programs.clone());
                                    }
                                }
                            },
                            Err(e) => {
                                warn!("did not have a latest run");
                                eprintln!("{e}");
                            }
                        },
                    }
                }

                info!("writing output");
                std::fs::write(output, input.to_string())?;
            }
            BatchCli::Publish { batch_path } => {
                use dioxus::prelude::*;

                let batch = Batch::from_path(batch_path)?;

                let mut groups_with_errors = vec![];

                for g in batch.groups.values() {
                    let span = span!(Level::INFO, "Group", name = g.config.name);
                    let _enter = span.enter();

                    let run = || -> Result<()> {
                        let data = match &*g.stage.read().unwrap() {
                            GroupStage::Initial => {
                                warn!("still haven't run");
                                return Ok(());
                            }
                            GroupStage::TestsRun {
                                pushed_results: true,
                                ..
                            } => return Ok(()),
                            GroupStage::TestsRun {
                                results,
                                pushed_results: false,
                            } => match results {
                                Ok(r) => r.clone(),
                                Err(e) => bail!("group had no results: {e:?}"),
                            },
                        };
                        let report = IndividualMarkdown {
                            programs_config: batch.programs.as_ref(),
                            group_name: g.config.name.clone(),
                            data,
                        };

                        let sh = Shell::new()?;

                        let temp_dir = sh.create_temp_dir()?;
                        sh.change_dir(temp_dir.path());

                        let row = ui::Row::from_group(g, false, true);
                        let html = dioxus_ssr::render_lazy(rsx!(ui::GroupRow {
                            row: row,
                            dedup: false,
                            open: true,
                        }));
                        sh.write_file("results.html", layout(&html))?;
                        sh.write_file("README.md", report.to_string())?;

                        let git_url = &g.config.git;

                        macro_rules! cmdq {
                                ($($t:tt)*) => {
                                    cmd!($($t)*).ignore_stdout().ignore_stderr().quiet()
                                };
                            }

                        cmdq!(sh, "git init").run()?;
                        set_checko_git_account(&sh)?;
                        cmdq!(sh, "git checkout -b results").run()?;
                        cmdq!(sh, "git add .").run()?;
                        cmdq!(sh, "git commit -m 'Update results'").run()?;
                        cmdq!(sh, "git remote add upstream {git_url}").run()?;
                        retry(
                            5.try_into().unwrap(),
                            Duration::from_millis(2000),
                            || match cmd!(sh, "git push --set-upstream upstream results --force")
                                .run()
                            {
                                Ok(_) => Ok(()),
                                Err(e) => {
                                    warn!("push failed...");
                                    Err(e)
                                }
                            },
                        )?;

                        info!("results pushed!");

                        match &mut *g.stage.write().unwrap() {
                            GroupStage::Initial => {}
                            GroupStage::TestsRun { pushed_results, .. } => *pushed_results = true,
                        }

                        batch.write_to_disk_sync()?;

                        std::thread::sleep(Duration::from_millis(2000));

                        Ok(())
                    };

                    if let Err(err) = run() {
                        error!("failed to push");
                        eprintln!("{err:?}");
                        groups_with_errors.push(g);
                    }
                }

                if !groups_with_errors.is_empty() {
                    error!("{} groups errored", groups_with_errors.len());
                    for g in groups_with_errors {
                        error!(group = g.config.name, "errored");
                    }
                }
            }
        }

        Ok(())
    }
}

fn layout(html: &str) -> String {
    format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta http-equiv="X-UA-Compatible" content="IE=edge">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Checko</title>
            <script src="https://cdn.tailwindcss.com"></script>
        </head>
        <body class="bg-slate-900 text-white">
            <div id="main">
                {html}
            </div>
        </body>
        </html>
    "#
    )
}
