use std::{path::PathBuf, str::FromStr};

fn main() {
    // Build the frontend using `just build-ui` iff we are building in release mode.
    if std::env::var("PROFILE").unwrap() == "release" {
        // TODO: we should be able to do this using `just`
        // let status = std::process::Command::new("just")
        //     .arg("build-ui")
        //     .stdout(std::process::Stdio::inherit())
        //     .stderr(std::process::Stdio::inherit())
        //     .status()
        //     .expect("Failed to build the frontend using `just build-ui`");
        // assert!(status.success());

        // make sure npm is in path
        if std::process::Command::new("npm")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_err()
        {
            #[cfg(target_os = "windows")]
            {
                eprintln!("npm is not installed. looking for it in C:/Program Files/nodejs/");

                // check if C:/Program Files/nodejs/ exists
                let Ok(nodejs_path) = PathBuf::from_str("C:/Program Files/nodejs/")
                    .unwrap()
                    .canonicalize()
                else {
                    eprintln!(
                        "nodejs is not installed. Please install it using `choco install nodejs`"
                    );
                    std::process::exit(1);
                };

                // add nodejs to path for windows
                std::env::set_var(
                    "PATH",
                    format!(
                        "{};{}",
                        std::env::var("PATH").unwrap(),
                        nodejs_path.display(),
                    ),
                );
            }
        }

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
