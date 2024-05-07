use std::str::FromStr;

use camino::Utf8PathBuf;
use chip::{ast_ext::SyntacticallyEquiv, model_check::ReachableStates};
use itertools::Itertools;

use crate::Groups;

pub struct TaskResultRow {
    name: String,
    git_hash: String,
    task: String,
    exists: bool,
    parse_error: bool,
    syntactically_equiv: bool,
    holds: u32,
    doesnt_hold: u32,
}

impl TaskResultRow {
    pub fn header() -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            "Group",
            "Git hash",
            "Task",
            "Exists",
            "Parse error",
            "Syntactically Equiv",
            "Holds",
            "Doesnt hold",
        )
    }
    pub fn as_csv(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.name,
            self.git_hash,
            self.task,
            self.exists,
            self.parse_error,
            self.syntactically_equiv,
            self.holds,
            self.doesnt_hold,
        )
    }
}

struct Test {
    program: chip::ast::LTLProgram,
    name: String,
}

pub async fn moka_check(
    reference: &Utf8PathBuf,
    groups: &Utf8PathBuf,
    tasks_dir: &String,
) -> Result<(), color_eyre::eyre::Error> {
    let reference = reference.canonicalize_utf8()?;
    tracing::info!(?reference);
    let mut tests: Vec<Test> = Vec::new();
    for e in std::fs::read_dir(&reference)? {
        let e = e?;
        if !e.file_type()?.is_file() {
            continue;
        }
        let path = Utf8PathBuf::from_path_buf(e.path().to_path_buf())
            .map_err(|p| color_eyre::Report::msg(format!("could not convert path: {:?}", p)))?;
        let src = std::fs::read_to_string(&path)?;
        let ltl_program = chip::parse::parse_ltl_program(&src)?;

        tests.push(Test {
            program: ltl_program,
            name: path.file_name().unwrap().to_string(),
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
                syntactically_equiv: false,
                holds: 0,
                doesnt_hold: 0,
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
                    syntactically_equiv: false,
                    holds: 0,
                    doesnt_hold: 0,
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

            let p = match chip::parse::parse_ltl_program(&src) {
                Ok(p) => p,
                Err(_e) => {
                    add_row(TaskResultRow {
                        name: g.name.clone(),
                        git_hash: git_hash.clone(),
                        task: t.name.clone(),
                        exists: true,
                        parse_error: true,
                        syntactically_equiv: false,
                        holds: 0,
                        doesnt_hold: 0,
                    });
                    continue;
                }
            };

            let is_syntactically_equiv = t
                .program
                .commands
                .iter()
                .is_syntactically_equiv(p.commands.iter());
            let Ok(rs) = ReachableStates::generate(&p, 10_000) else {
                todo!()
            };

            let mut holds = 0;
            let mut doesnt_hold = 0;
            for (_, property) in &t.program.properties {
                let pl = rs.pipeline(property);
                if pl.product_ba().find_accepting_cycle().is_some() {
                    doesnt_hold += 1
                } else {
                    holds += 1
                }
            }
            add_row(TaskResultRow {
                name: g.name.clone(),
                git_hash: git_hash.clone(),
                task: t.name.clone(),
                exists: true,
                parse_error: false,
                syntactically_equiv: is_syntactically_equiv,
                holds,
                doesnt_hold,
            });
        }
    }
    Ok(())
}
