//! Formatting of primarily Markdown files.

use std::{
    cmp::{self, Reverse},
    collections::BTreeMap,
    time::Duration,
};

use checkr::env::Analysis;
use itertools::Itertools;

use crate::{
    config::CanonicalProgramsConfig,
    test_runner::{TestResult, TestResultType, TestRunData, TestRunResults},
};

#[derive(Debug)]
pub struct IndividualMarkdown<'a> {
    pub programs_config: &'a CanonicalProgramsConfig,
    pub group_name: String,
    pub data: TestRunResults,
}

impl std::fmt::Display for IndividualMarkdown<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# {}", self.group_name)?;

        match &self.data.data {
            TestRunData::CompileError(msg) => {
                writeln!(f, "## Failed to compile")?;
                writeln!(f)?;
                writeln!(f, "```")?;
                writeln!(f, "{}", msg.trim())?;
                writeln!(f, "```")?;
            }
            TestRunData::Sections(sections) => {
                for sec in sections {
                    writeln!(f, "## {}", sec.analysis)?;

                    let mut table = comfy_table::Table::new();
                    table
                        .load_preset(comfy_table::presets::ASCII_MARKDOWN)
                        .set_header(["Program", "Result", "Time", "Link"]);

                    for (idx, summary) in sec.programs.iter().enumerate() {
                        table.add_row([
                            format!("Program {}", idx + 1),
                            match &summary.result {
                                TestResultType::CorrectTerminated => "Correct",
                                TestResultType::CorrectNonTerminated { .. } => {
                                    "Correct<sup>*</sup>"
                                }
                                TestResultType::Mismatch { .. } => "Mismatch",
                                TestResultType::TimeOut => "Time out",
                                TestResultType::Error { .. } => "Error",
                            }
                            .to_string(),
                            format!("{:?}", summary.time),
                            if summary.shown {
                                let mut target = String::new();
                                let mut serializer =
                                    url::form_urlencoded::Serializer::new(&mut target);
                                let program =
                                    self.programs_config.get(summary.analysis, summary.id);
                                serializer
                                    .append_pair("analysis", sec.analysis.command())
                                    .append_pair("src", &program.src)
                                    .append_pair("input", &program.input);
                                format!("[Link](http://localhost:3000/?{target})")
                            } else {
                                "Hidden".to_string()
                            },
                        ]);
                    }
                    writeln!(f, "\n{table}")?;
                }

                let mut table = comfy_table::Table::new();
                table
                    .load_preset(comfy_table::presets::ASCII_MARKDOWN)
                    .set_header(["Result", "Explanation"])
                    .add_row(["Correct", "Nice job! :)"])
                    .add_row([
                        "Correct<sup>*</sup>",
                        "The program ran correctly for a limited number of steps",
                    ])
                    .add_row(["Mismatch", "The result did not match the expected output"])
                    .add_row(["Error", "Unable to parse the output"]);
                writeln!(f, "\n## Result explanations")?;
                writeln!(f, "\n{table}")?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompetitionListing<T = Vec<TestResult>> {
    Results(T),
    CompileError,
    CriticalError,
}

impl From<Vec<TestResult>> for CompetitionListing {
    fn from(value: Vec<TestResult>) -> Self {
        CompetitionListing::Results(value)
    }
}

#[derive(Debug, Default)]
pub struct CompetitionMarkdown {
    pub sections: BTreeMap<Analysis, BTreeMap<String, CompetitionListing>>,
}

impl std::fmt::Display for CompetitionMarkdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut num_tests = 0;
        for (analysis, groups) in &self.sections {
            let sorted_groups = groups
                .iter()
                .map(|(g, test_results)| {
                    let num_correct_rev = match test_results {
                        CompetitionListing::Results(results) => {
                            num_tests = cmp::max(num_tests, results.len());
                            CompetitionListing::Results(Reverse(
                                results.iter().filter(|t| t.result.is_correct()).count(),
                            ))
                        }
                        CompetitionListing::CompileError => CompetitionListing::CompileError,
                        CompetitionListing::CriticalError => CompetitionListing::CriticalError,
                    };
                    let time: Duration = match test_results {
                        CompetitionListing::Results(results) => {
                            results.iter().map(|t| t.time).sum()
                        }
                        _ => Duration::MAX,
                    };
                    (num_correct_rev, time, g)
                })
                .sorted();

            writeln!(f, "## {analysis}")?;

            let mut table = comfy_table::Table::new();
            table
                .load_preset(comfy_table::presets::ASCII_MARKDOWN)
                .set_header(["Rank", "Group", "Result", "Time"]);

            for (rank_0, (data, time, g)) in sorted_groups.enumerate() {
                let (data, time) = match data {
                    CompetitionListing::Results(Reverse(num_correct)) => (
                        format!("{num_correct}/{num_tests} passed"),
                        format!("{time:?}"),
                    ),
                    CompetitionListing::CompileError => {
                        ("Compile error*".to_string(), "---".to_string())
                    }
                    CompetitionListing::CriticalError => {
                        ("Critical error*".to_string(), "---".to_string())
                    }
                };
                table.add_row([format!("{}", rank_0 + 1), g.to_string(), data, time]);
            }

            writeln!(f, "\n{table}\n")?;
        }

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(["Error", "Explanation"])
            .add_row([
                "Compile error",
                "Your code failed to compile. \
                 Check the `results` branch on your repository for details.",
            ])
            .add_row([
                "Critical error",
                "This was caused by a critical error during your test run. \
                 Often this is the result of producing too much output and \
                 overflowing the internal buffers.",
            ]);
        writeln!(f, "\n## Error explanations")?;
        writeln!(f, "\n{table}")?;

        Ok(())
    }
}
