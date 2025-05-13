use std::path::PathBuf;

use serde::Deserialize;
use xshell::cmd;

use crate::{Result, project_root};

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub fn z3_versions_dir() -> PathBuf {
    project_root().parent().unwrap().join("z3-versions")
}

pub async fn run(target: &str) -> Result<Vec<String>> {
    let sh = xshell::Shell::new()?;
    sh.change_dir(project_root().parent().unwrap());

    let versions_dir = z3_versions_dir();
    let tmp_dir = versions_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;
    sh.change_dir(&tmp_dir);

    let versions = get_latest_versions().await?;
    for version in versions.iter().take(10) {
        println!("Found release: {}", version.tag_name);
        for asset in &version.assets {
            if asset.name.contains(target) {
                println!("Found asset: {}", asset.name);
                // println!("Downloading {}", asset.browser_download_url);
                let dst = tmp_dir.join(&asset.name);
                let url = &asset.browser_download_url;
                cmd!(sh, "curl -L --compressed -o {dst} {url}").run()?;
                cmd!(sh, "unzip -o {dst}").run()?;
                let target = versions_dir.join(&version.tag_name);
                std::fs::create_dir_all(&target)?;
                std::fs::copy(
                    dst.with_extension("").join("bin").join("z3"),
                    target.join("z3"),
                )?;
            }
        }
    }

    std::fs::remove_dir_all(&tmp_dir)?;

    // let sh = xshell::Shell::new()?;
    // let mut releases = Vec::new();
    // for version in versions {
    //     let url = format!(
    //         "https://api.github.com/repos/Z3Prover/z3/releases/tags/{}",
    //         version
    //     );
    //     let response: Release = reqwest::blocking::get(&url)?.json()?;
    //     releases.push(response);
    // }

    // for release in releases {
    //     println!("Found release: {}", release.tag_name);
    //     for asset in release.assets {
    //         println!("Found asset: {}", asset.name);
    //         if asset.name.ends_with(".tar.gz") {
    //             // println!("Downloading {}", asset.browser_download_url);
    //             // cmd!(sh, "curl -L -o {}/{} {asset.browser_download_url}",
    //             // project_root(), asset.name).run()?;
    //         }
    //     }
    // }

    Ok(versions.iter().map(|v| v.tag_name.clone()).collect())
}

async fn get_latest_versions() -> Result<Vec<Release>> {
    let releases_url = "https://api.github.com/repos/Z3Prover/z3/releases";
    let client = reqwest::Client::new();
    let res: Vec<Release> = client
        .get(releases_url)
        .header("User-Agent", "reqwest")
        .send()
        .await?
        .json()
        .await?;
    Ok(res)
}
