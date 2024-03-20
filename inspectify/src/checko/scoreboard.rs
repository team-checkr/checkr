use std::collections::HashMap;

use ce_shell::{Analysis, Input};
use driver::JobState;
use itertools::{Either, Itertools};

use super::{config::GroupName, Checko, GroupStatus};

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
    status: GroupStatus,
    last_hash: Option<String>,
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

// TODO: Perhaps we should split events up into more selective changes
// #[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
// pub enum PublicEvent {
//     LastFinished(Option<chrono::DateTime<chrono::FixedOffset>>),
//     Analysis(Analysis, Vec<Option<Input>>),
//     Group(GroupName, Vec<PublicAnalysisResults>),
//     GroupOrder(Vec<GroupName>),
// }

async fn compute_public_groups(checko: &Checko) -> Vec<PublicGroup> {
    let mut groups = HashMap::<GroupName, PublicGroup>::new();

    for (group_name, analysis, gs) in checko.group_states().await {
        let pg = groups
            .entry(group_name.clone())
            .or_insert_with(|| PublicGroup {
                name: group_name.clone(),
                analysis_results: vec![],
            });

        let gs_results = gs.results().await;
        let results = checko
            .programs_config
            .inputs()
            .flat_map(|(a, inputs)| {
                if a != analysis {
                    Either::Left(std::iter::empty())
                } else {
                    Either::Right(inputs.map(|input| {
                        PublicProgramResult {
                            state: gs_results
                                .get(&input.hash())
                                .cloned()
                                .unwrap_or(JobState::Queued),
                        }
                    }))
                }
            })
            .collect();

        pg.analysis_results.push(PublicAnalysisResults {
            analysis,
            status: gs.status().await,
            last_hash: gs.latest_hash().await,
            results,
        });
    }

    groups.into_values().collect()
}

pub async fn compute_public_state(checko: &Checko) -> PublicState {
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
    let groups = compute_public_groups(checko)
        .await
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
                g.name.clone(),
            ))
        })
        .collect();
    PublicState {
        last_finished: checko.last_finished(),
        analysis,
        groups,
    }
}
