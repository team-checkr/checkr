fn main() {
    // Build the frontend using `just build-ui` iff we are building in release mode.
    if std::env::var("PROFILE").unwrap() == "release" {
        let status = std::process::Command::new("just")
            .arg("build-ui")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .expect("Failed to build the frontend using `just build-ui`");
        assert!(status.success());
    }
}
