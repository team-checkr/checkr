use std::{path::PathBuf, str::FromStr};

fn main() {
    // Build the frontend using `just build-ui` iff we are building in release mode.
    if std::env::var("PROFILE").unwrap() == "release" {
        // run the equivilent `cd apps/inspectify && (npm install && npm run build)`
        let inspectify_root =
            PathBuf::from_str(std::env::var("CARGO_MANIFEST_DIR").unwrap().as_str())
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("apps/inspectify")
                .canonicalize()
                .unwrap();

        let status = std::process::Command::new("npm")
            .current_dir(&inspectify_root)
            .arg("install")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .expect("Failed to install the frontend using `npm install`");
        assert!(status.success());

        let status = std::process::Command::new("npm")
            .current_dir(&inspectify_root)
            .arg("run")
            .arg("build")
            .env("PUBLIC_API_BASE", "")
            .env("PUBLIC_CHECKO", "")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .expect("Failed to build the frontend using `npm run build`");
        assert!(status.success());
    }
}
