use std::{str::FromStr, time::Duration};

use camino::Utf8PathBuf;
use chip::ast_ext::SyntacticallyEquiv;
use color_eyre::eyre::Context;
use itertools::Itertools;

use crate::Groups;

pub struct TaskResultRow {
    name: String,
    git_hash: String,
    task: String,
    exists: bool,
    parse_error: bool,
    is_fully_annotated: bool,
    syntactically_equiv: bool,
    num_unsat: u32,
    num_sat: u32,
    num_unknown: u32,
    num_timeout: u32,
}

impl TaskResultRow {
    pub fn header() -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            "Group",
            "Git hash",
            "Task",
            "Exists",
            "Parse error",
            "Fully Annotated",
            "Syntactically Equiv",
            "Unsat",
            "Sat",
            "Unknown",
            "Timeout"
        )
    }
    pub fn as_csv(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.name,
            self.git_hash,
            self.task,
            self.exists,
            self.parse_error,
            self.is_fully_annotated,
            self.syntactically_equiv,
            self.num_unsat,
            self.num_sat,
            self.num_unknown,
            self.num_timeout
        )
    }
}

struct Test {
    program: chip::ast::AGCLCommands,
    name: String,
    pre: Option<chip::ast::PredicateBlock>,
    post: Option<chip::ast::PredicateBlock>,
}

pub async fn chip_check(
    reference: &Utf8PathBuf,
    groups: &Utf8PathBuf,
    tasks_dir: &String,
) -> Result<(), color_eyre::eyre::Error> {
    tracing::debug!(?reference, ?groups, ?tasks_dir, "running chip_check");
    let reference = reference.canonicalize_utf8()?;
    tracing::info!(?reference, "canonicalized reference dir");
    let mut tests: Vec<Test> = Vec::new();
    for e in std::fs::read_dir(&reference)? {
        let e = e?;
        if !e.file_type()?.is_file() {
            continue;
        }
        // only accept .gcl files
        if !e.file_name().to_string_lossy().ends_with(".gcl") {
            continue;
        }
        let path = Utf8PathBuf::from_path_buf(e.path().to_path_buf())
            .map_err(|p| color_eyre::Report::msg(format!("could not convert path: {p:?}")))?;
        tracing::debug!(?path, "reading test");
        let src =
            std::fs::read_to_string(&path).with_context(|| format!("failed to read {path}"))?;
        let program = chip::parse::parse_agcl_program(&src)
            .with_context(|| format!("failed to parse {path}"))?;
        let pre = program
            .0
            .first()
            .and_then(|cmd| cmd.pre.predicates.first())
            .cloned();
        let post = program
            .0
            .last()
            .and_then(|cmd| cmd.post.predicates.last())
            .cloned();
        tests.push(Test {
            program,
            name: path.file_name().unwrap().to_string(),
            pre,
            post,
        });
    }
    let groups = std::fs::read_to_string(groups)?;
    let groups: Groups = toml::from_str(&groups)?;
    tracing::info!(?groups);
    let working_dir = Utf8PathBuf::from_str("working")?;
    std::fs::create_dir_all(&working_dir)?;
    let working_dir = working_dir.canonicalize_utf8()?;
    tracing::info!(?working_dir);
    let mut rows: Vec<TaskResultRow> = Vec::new();
    println!("{}", TaskResultRow::header());
    let mut add_row = |row: TaskResultRow| {
        println!("{}", row.as_csv());
        rows.push(row);
    };
    for (g_idx, g) in groups.groups.iter().enumerate() {
        let span = tracing::info_span!(
            "group",
            name = %g.name,
            p = format!("{}/{}", g_idx+1, groups.groups.len()),
        );
        let _e = span.enter();
        tracing::info!("cloning");
        let g_dir = working_dir.join(&g.name);
        std::fs::create_dir_all(&g_dir)?;
        let g_dir = g_dir.canonicalize_utf8()?;
        gitty::clone_or_pull(&g.git, &g_dir).await?;

        let git_hash = gitty::hash(&g_dir, None).await?;

        let Ok(tasks_dir) = g_dir.join(tasks_dir).canonicalize_utf8() else {
            tracing::error!("did not have tasks");
            add_row(TaskResultRow {
                name: g.name.clone(),
                git_hash,
                task: "".to_string(),
                exists: false,
                parse_error: false,
                is_fully_annotated: false,
                syntactically_equiv: false,
                num_unsat: 0,
                num_sat: 0,
                num_unknown: 0,
                num_timeout: 0,
            });
            continue;
        };
        for (t_idx, t) in tests.iter().enumerate() {
            let span = tracing::info_span!(
                "task",
                name = t.name,
                p = format!("{}/{}", t_idx + 1, tests.len()),
            );
            let _e = span.enter();
            tracing::info!("testing");

            let Ok(path) = tasks_dir.join(&t.name).canonicalize_utf8() else {
                tracing::error!("did not have task");
                add_row(TaskResultRow {
                    name: g.name.clone(),
                    git_hash: git_hash.clone(),
                    task: t.name.clone(),
                    exists: false,
                    parse_error: false,
                    is_fully_annotated: false,
                    syntactically_equiv: false,
                    num_unsat: 0,
                    num_sat: 0,
                    num_unknown: 0,
                    num_timeout: 0,
                });
                continue;
            };

            let src = std::fs::read_to_string(path)?;
            // strip all comments
            let src = src
                .lines()
                .map(|l| {
                    if let Some(i) = l.find("//") {
                        &l[..i]
                    } else {
                        l
                    }
                })
                .join("\n");
            // remove everything leading up to the first {
            let src = src.trim_start_matches(|c| c != '{');
            // remove everything after the last }
            let src = src.trim_end_matches(|c| c != '}');

            let mut p = match chip::parse::parse_agcl_program(src) {
                Ok(p) => p,
                Err(_e) => {
                    add_row(TaskResultRow {
                        name: g.name.clone(),
                        git_hash: git_hash.clone(),
                        task: t.name.clone(),
                        exists: true,
                        parse_error: true,
                        is_fully_annotated: false,
                        syntactically_equiv: false,
                        num_unsat: 0,
                        num_sat: 0,
                        num_unknown: 0,
                        num_timeout: 0,
                    });
                    continue;
                }
            };

            // Wrap p in pre and post from t
            if let (Some(pre), Some(cmd)) = (&t.pre, p.0.first_mut()) {
                cmd.pre.predicates.insert(0, pre.clone());
            }
            if let (Some(post), Some(cmd)) = (&t.post, p.0.last_mut()) {
                cmd.post.predicates.push(post.clone());
            }

            let is_syntactically_equiv = t.program.is_syntactically_equiv(&p);
            let is_fully_annotated = p.is_fully_annotated();
            let mut num_unsat = 0;
            let mut num_sat = 0;
            let mut num_unknown = 0;
            let mut num_timeout = 0;

            for result in chip_chip(DEFAULT_TIMEOUT, &p).await? {
                match result.result {
                    AssertionResultKind::Sat => num_sat += 1,
                    AssertionResultKind::Unsat => num_unsat += 1,
                    AssertionResultKind::Unknown => num_unknown += 1,
                    AssertionResultKind::Timeout => num_timeout += 1,
                }
            }
            add_row(TaskResultRow {
                name: g.name.clone(),
                git_hash: git_hash.clone(),
                task: t.name.clone(),
                exists: true,
                parse_error: false,
                is_fully_annotated,
                syntactically_equiv: is_syntactically_equiv,
                num_unsat,
                num_sat,
                num_unknown,
                num_timeout,
            });
        }
    }
    Ok(())
}

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);

pub async fn chip_chip(
    timeout: Duration,
    p: &chip::ast::AGCLCommands,
) -> Result<Vec<AssertionResult>, color_eyre::eyre::Error> {
    let st = smtlib::Storage::new();
    let prelude = smtlib::lowlevel::ast::Script::parse(&st, &p.prelude())?;

    let mut assertion_results: Vec<AssertionResult> = Vec::new();
    for assertion in p.assertions() {
        let backend = smtlib::backend::z3_binary::tokio::Z3BinaryTokio::new("z3")
            .await
            .with_context(|| "failed to create z3 backend")?;
        let mut solver = smtlib::TokioSolver::new(&st, backend).await?;
        for cmd in prelude.0.iter() {
            solver.run_command(*cmd).await?;
        }
        let a = !assertion.predicate.smt(&st);
        tracing::debug!(%a, "asserting");
        solver.assert(a).await?;

        let res = tokio::time::timeout(timeout, solver.check_sat()).await;

        match res {
            Ok(res) => {
                let kind = match res? {
                    smtlib::SatResult::Unsat => AssertionResultKind::Unsat,
                    smtlib::SatResult::Sat => AssertionResultKind::Sat,
                    smtlib::SatResult::Unknown => AssertionResultKind::Unknown,
                };
                assertion_results.push(AssertionResult {
                    assertion: assertion.clone(),
                    result: kind,
                });
            }
            Err(_) => {
                assertion_results.push(AssertionResult {
                    assertion: assertion.clone(),
                    result: AssertionResultKind::Timeout,
                });
            }
        }
    }
    Ok(assertion_results)
}

pub struct AssertionResult {
    pub assertion: chip::triples::Assertion,
    pub result: AssertionResultKind,
}

pub enum AssertionResultKind {
    Sat,
    Unsat,
    Unknown,
    Timeout,
}
