use std::{collections::HashSet, fs, str::FromStr};

use crate::endpoints::InspectifyJobMeta;
use ce_core::Generate;
use ce_shell::Analysis;
use driver::JobKind;
use gcl::pg::Determinism;
use rand::{SeedableRng, seq::IndexedRandom};
use roxmltree::Document;

fn covered_lines(xml: &str, file_filter: &str) -> HashSet<(String, u32)> {
    let doc = Document::parse(xml).unwrap();
    let mut covered = HashSet::new();

    for class in doc.descendants().filter(|n| n.has_tag_name("class")) {
        let filename = class.attribute("filename").unwrap_or("").to_string();
        if !filename.contains(file_filter) {
            continue;
        }
        for line in class
            .children()
            .filter(|n| n.has_tag_name("lines"))
            .flat_map(|ls| ls.children().filter(|n| n.has_tag_name("line")))
        {
            let hits: u64 = line.attribute("hits").unwrap_or("0").parse().unwrap_or(0);
            if hits > 0 {
                let number: u32 = line.attribute("number").unwrap_or("0").parse().unwrap_or(0);
                covered.insert((filename.clone(), number));
            }
        }
    }

    covered
}

fn total_lines_in_file(xml: &str, file_filter: &str) -> usize {
    let doc = Document::parse(xml).unwrap();
    let mut total = 0;
    for class in doc.descendants().filter(|n| n.has_tag_name("class")) {
        let filename = class.attribute("filename").unwrap_or("");
        if !filename.contains(file_filter) {
            continue;
        }
        for _ in class
            .children()
            .filter(|n| n.has_tag_name("lines"))
            .flat_map(|ls| ls.children().filter(|n| n.has_tag_name("line")))
        {
            total += 1;
        }
    }
    total
}

fn all_tracked_lines(xml: &str, file_filter: &str) -> HashSet<(String, u32)> {
    let doc = Document::parse(xml).unwrap();
    let mut tracked = HashSet::new();
    for class in doc.descendants().filter(|n| n.has_tag_name("class")) {
        let filename = class.attribute("filename").unwrap_or("").to_string();
        if !filename.contains(file_filter) {
            continue;
        }
        for line in class
            .children()
            .filter(|n| n.has_tag_name("lines"))
            .flat_map(|ls| ls.children().filter(|n| n.has_tag_name("line")))
        {
            let number: u32 = line.attribute("number").unwrap_or("0").parse().unwrap_or(0);
            tracked.insert((filename.clone(), number));
        }
    }
    tracked
}

fn print_uncovered_lines(uncovered: &HashSet<(String, u32)>, cwd: &std::path::Path, label: &str) {
    if uncovered.is_empty() {
        println!("  [{label}] All lines covered!");
        return;
    }
    let mut by_file: std::collections::BTreeMap<String, Vec<u32>> =
        std::collections::BTreeMap::new();
    for (filename, line_no) in uncovered {
        by_file.entry(filename.clone()).or_default().push(*line_no);
    }
    for (filename, mut line_nos) in by_file {
        line_nos.sort();
        let source_lines: Vec<String> = fs::read_to_string(cwd.join(&filename))
            .map(|s| s.lines().map(|l| l.to_string()).collect())
            .unwrap_or_default();
        println!("  [{label}] Uncovered lines in {filename}:");
        for ln in &line_nos {
            let content = source_lines
                .get((*ln as usize).saturating_sub(1))
                .map(|s| s.trim())
                .unwrap_or("<unknown>");
            println!("    line {:>4}: {content}", ln);
        }
    }
}

/// Runs dotnet-coverage for `test_amount` seeds using the provided arg generator.
/// Returns (unique_lines_hit, total_instrumentable_lines).
/// Unique lines = union of all lines hit across all seeds.
async fn coverage_test(
    hub: &driver::Hub<InspectifyJobMeta>,
    cwd: &std::path::PathBuf,
    driver: &driver::Driver<InspectifyJobMeta>,
    label: &str,
    file_filter: &str,
    test_amount: usize,
    mut get_args: impl FnMut(usize) -> (String, String),
) -> (usize, usize, HashSet<(String, u32)>) {
    let run_exe = cwd.join(driver.config().run().split(' ').next().unwrap());
    let run_exe_str = run_exe.to_string_lossy().into_owned();

    let mut union_covered: HashSet<(String, u32)> = HashSet::new();
    let mut all_tracked: HashSet<(String, u32)> = HashSet::new();
    let mut total_possible = 0usize;

    for index in 1..=test_amount {
        let (program, args) = get_args(index);

        let job = hub.exec_command(
            JobKind::Compilation,
            cwd.clone(),
            InspectifyJobMeta::default(),
            "dotnet-coverage",
            [
                "collect",
                "--output-format",
                "cobertura",
                "--output",
                "coverage.xml",
                run_exe_str.as_str(),
                program.as_str(),
                args.as_str(),
            ],
        );

        job.wait().await;

        let xml_path = cwd.join("coverage.xml");
        let xml = fs::read_to_string(&xml_path).expect("coverage.xml not found");

        if all_tracked.is_empty() {
            all_tracked = all_tracked_lines(&xml, file_filter);
        }

        if total_possible == 0 {
            total_possible = total_lines_in_file(&xml, file_filter);
        }

        let this_run = covered_lines(&xml, file_filter);
        let new_this_seed = this_run.difference(&union_covered).count();
        union_covered.extend(this_run);

        print!(
            "\r  [{label}] {index}/{test_amount} — {file_filter}: {}/{total_possible} lines (+{new_this_seed} new)",
            union_covered.len()
        );
    }
    println!();
    let uncovered: HashSet<(String, u32)> =
        all_tracked.difference(&union_covered).cloned().collect();
    (union_covered.len(), total_possible, uncovered)
}

/// Serialize only the fields F#'s Io.Compiler.Input expects: { commands, determinism }.
fn to_fsharp_compiler_json(commands: &gcl::ast::Commands, determinism: Determinism) -> String {
    let commands_str = commands.to_string();
    let det_str = match determinism {
        Determinism::Deterministic => "Deterministic",
        Determinism::NonDeterministic => "NonDeterministic",
    };
    format!(
        r#"{{"commands":{},"determinism":"{}"}}"#,
        serde_json::to_string(&commands_str).unwrap(),
        det_str
    )
    .replace('"', "\\\"")
}

/// Generate a Compiler input using the OLD gcl_gen
fn compiler_input_old_gen(seed: usize) -> (String, String) {
    let mut rng = rand::rngs::SmallRng::seed_from_u64(seed as u64);
    let commands = gcl::ast::Commands::gn(&mut Default::default(), &mut rng);
    let determinism = *[Determinism::Deterministic, Determinism::NonDeterministic]
        .choose(&mut rng)
        .unwrap();
    (
        "Compiler".to_string(),
        to_fsharp_compiler_json(&commands, determinism),
    )
}

/// Generate a Compiler input using the NEW gcl_compiler_gen
fn compiler_input_new_gen(seed: usize) -> (String, String) {
    use ce_core::gn::compiler_gen::{CompilerContext, gen_commands};
    let mut rng = rand::rngs::SmallRng::seed_from_u64(seed as u64);
    let commands = gen_commands(
        &mut CompilerContext {
            fuel: 30,
            ..Default::default()
        },
        &mut rng,
    );
    let determinism = *[Determinism::Deterministic, Determinism::NonDeterministic]
        .choose(&mut rng)
        .unwrap();
    (
        "Compiler".to_string(),
        to_fsharp_compiler_json(&commands, determinism),
    )
}

struct RepoResult {
    name: String,
    old_unique: usize,
    old_total: usize,
    new_unique: usize,
    new_total: usize,
    old_uncovered: HashSet<(String, u32)>,
    new_uncovered: HashSet<(String, u32)>,
}

#[tokio::test]
async fn test_thingy() {
    // Prerequisites:
    //   dotnet tool install -g dotnet-coverage
    //   Each repo must already be compiled: dotnet publish -c Release --self-contained --output bin/run

    let student_repos_root = "D:/checkr/Student-repos-for-testing";
    let test_amount = 50;

    // Discover all repos: any subdirectory that contains a code/run.toml
    let mut repo_paths: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(student_repos_root)
        .expect("could not read Student-repos-for-testing")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let code_path = entry.path().join("code");
            if code_path.join("run.toml").exists() {
                Some((name, code_path))
            } else {
                None
            }
        })
        .collect();
    repo_paths.sort_by(|a, b| a.0.cmp(&b.0));

    assert!(
        !repo_paths.is_empty(),
        "no student repos found in {student_repos_root}"
    );

    let hub: driver::Hub<InspectifyJobMeta> = driver::Hub::new().expect("hub init failed");
    let mut results: Vec<RepoResult> = Vec::new();

    for (name, code_path) in &repo_paths {
        println!("\n{}", "=".repeat(60));
        println!("=== Repo: {name}");
        println!("{}", "=".repeat(60));

        let cwd = dunce::canonicalize(code_path)
            .unwrap_or_else(|_| panic!("could not canonicalize {}", code_path.display()));
        let run_toml = cwd.join("run.toml");

        let driver = driver::Driver::new_from_path(hub.clone(), &cwd, run_toml)
            .unwrap_or_else(|e| panic!("driver init failed for {name}: {e}"));

        println!("  Compiling...");
        if let Some(job) = driver.ensure_compile(InspectifyJobMeta::default()) {
            job.wait().await;
        }

        println!("\n  --- Old gcl_gen ({test_amount} seeds) ---");
        let (old_unique, old_total, old_uncovered) = coverage_test(
            &hub,
            &cwd,
            &driver,
            "old gcl_gen",
            "Compiler.fs",
            test_amount,
            compiler_input_old_gen,
        )
        .await;

        println!("\n  --- New gcl_compiler_gen ({test_amount} seeds) ---");
        let (new_unique, new_total, new_uncovered) = coverage_test(
            &hub,
            &cwd,
            &driver,
            "new gcl_gen",
            "Compiler.fs",
            test_amount,
            compiler_input_new_gen,
        )
        .await;

        results.push(RepoResult {
            name: name.clone(),
            old_unique,
            old_total,
            new_unique,
            new_total,
            old_uncovered,
            new_uncovered,
        });
    }

    // Final summary table
    println!("\n\n{}", "=".repeat(80));
    println!("=== SUMMARY ({test_amount} seeds each generator) ===");
    println!(
        "{:<45} {:>12} {:>12} {:>10}",
        "Repo", "Old %", "New %", "Delta"
    );
    println!("{}", "-".repeat(80));
    for r in &results {
        let old_pct = r.old_unique as f64 / r.old_total as f64 * 100.0;
        let new_pct = r.new_unique as f64 / r.new_total as f64 * 100.0;
        let delta = new_pct - old_pct;
        println!(
            "{:<45} {:>11.2}% {:>11.2}% {:>+10.2}%",
            r.name, old_pct, new_pct, delta
        );
    }
    println!("{}", "-".repeat(80));

    // Per-repo uncovered line breakdown
    println!("\n\n{}", "=".repeat(80));
    println!("=== UNCOVERED LINES BREAKDOWN ===");
    for r in &results {
        let cwd = dunce::canonicalize(
            std::path::Path::new(student_repos_root)
                .join(&r.name)
                .join("code"),
        )
        .unwrap_or_else(|_| {
            std::path::Path::new(student_repos_root)
                .join(&r.name)
                .join("code")
                .to_path_buf()
        });

        println!("\n--- {} ---", r.name);
        print_uncovered_lines(&r.old_uncovered, &cwd, "old gcl_gen");
        print_uncovered_lines(&r.new_uncovered, &cwd, "new gcl_gen");

        let only_old: HashSet<_> = r
            .new_uncovered
            .difference(&r.old_uncovered)
            .cloned()
            .collect();
        let only_new: HashSet<_> = r
            .old_uncovered
            .difference(&r.new_uncovered)
            .cloned()
            .collect();
        if !only_old.is_empty() {
            print_uncovered_lines(&only_old, &cwd, "REGRESSION: new gen misses (old hit)");
        }
        if !only_new.is_empty() {
            print_uncovered_lines(&only_new, &cwd, "IMPROVEMENT: new gen hits (old missed)");
        }
    }
}
