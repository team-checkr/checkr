use ce_core::ValidationResult;
use ce_shell::{Analysis, Input};
use driver::{JobKind, JobState};
use itertools::Itertools;

use crate::endpoints::InspectifyJobMeta;

use super::{config::GroupName, Checko};

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
pub struct PublicAnalysis {
    analysis: Analysis,
    programs: Vec<Option<Input>>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
pub struct PublicGroup {
    name: GroupName,
    analysis_results: Vec<PublicAnalysisResults>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
pub struct PublicAnalysisResults {
    analysis: Analysis,
    results: Vec<PublicProgramResult>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
pub struct PublicProgramResult {
    state: JobState,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
pub struct PublicState {
    last_finished: Option<chrono::DateTime<chrono::FixedOffset>>,
    analysis: Vec<PublicAnalysis>,
    groups: Vec<PublicGroup>,
}

fn compute_public_groups(
    hub: &driver::Hub<InspectifyJobMeta>,
    checko: &Checko,
) -> Vec<PublicGroup> {
    let jobs = hub.jobs(None);
    let groups = checko.groups_config().groups.iter().map(|group| {
        let analysis_results = checko
            .programs_config()
            .envs
            .iter()
            .map(|(analysis, ps)| {
                let results = ps
                    .programs
                    .iter()
                    .map(|p| {
                        let input = analysis.input_from_str(&p.input).unwrap();
                        let job = jobs.iter().find(|j| {
                            j.meta().group_name.as_deref() == Some(group.name.as_str())
                                && j.kind() == JobKind::Analysis(input.clone())
                        });
                        let state = job
                            .map(|j| {
                                let output = input.analysis().output_from_str(&j.stdout());
                                let validation = match (j.state(), &output) {
                                    (JobState::Succeeded, Ok(output)) => {
                                        Some(match input.validate_output(output) {
                                            Ok(output) => output,
                                            Err(e) => ValidationResult::Mismatch {
                                                reason: format!("failed to validate output: {e:?}"),
                                            },
                                        })
                                    }
                                    (JobState::Succeeded, Err(e)) => {
                                        Some(ValidationResult::Mismatch {
                                            reason: format!("failed to parse output: {e:?}"),
                                        })
                                    }
                                    _ => None,
                                };

                                match (j.state(), validation) {
                                    (
                                        JobState::Succeeded,
                                        Some(
                                            ValidationResult::CorrectNonTerminated { .. }
                                            | ValidationResult::CorrectTerminated,
                                        ),
                                    ) => JobState::Succeeded,
                                    (
                                        JobState::Succeeded,
                                        Some(ValidationResult::Mismatch { .. }),
                                    ) => JobState::Warning,
                                    (JobState::Succeeded, Some(ValidationResult::TimeOut)) => {
                                        JobState::Timeout
                                    }
                                    (state, _) => state,
                                }
                            })
                            .unwrap_or(JobState::Queued);
                        (PublicProgramResult { state }, input.hash())
                    })
                    .sorted_by_key(|(_, hash)| *hash)
                    .map(|(res, _)| res);
                PublicAnalysisResults {
                    analysis: *analysis,
                    results: results.collect(),
                }
            })
            .collect();
        PublicGroup {
            name: group.name.clone(),
            analysis_results,
        }
    });
    groups.collect()
}

pub fn compute_public_state(hub: &driver::Hub<InspectifyJobMeta>, checko: &Checko) -> PublicState {
    let analysis = checko
        .programs_config()
        .envs
        .iter()
        .map(|(analysis, ps)| PublicAnalysis {
            analysis: *analysis,
            programs: ps
                .programs
                .iter()
                .map(|p| {
                    let input = analysis.input_from_str(&p.input).unwrap();
                    Some(input)
                })
                .collect(),
        })
        .collect();
    let groups = compute_public_groups(hub, checko)
        .into_iter()
        .sorted_by_key(|g| {
            std::cmp::Reverse((
                g.analysis_results
                    .iter()
                    .flat_map(|a| a.results.iter().filter(|x| x.state == JobState::Succeeded))
                    .count(),
                g.analysis_results
                    .iter()
                    .flat_map(|a| a.results.iter().filter(|x| x.state == JobState::Warning))
                    .count(),
            ))
        })
        .collect();
    PublicState {
        last_finished: checko.last_finished(),
        analysis,
        groups,
    }
}
