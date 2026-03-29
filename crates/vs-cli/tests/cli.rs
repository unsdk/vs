use std::fs;
use std::path::Path;
use std::process::Command;

use vs_test_support::{fixture_root, temp_workspace};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_vs")
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> std::process::Output {
    match Command::new(binary())
        .args(args)
        .env("VS_HOME", home)
        .current_dir(cwd)
        .output()
    {
        Ok(output) => output,
        Err(error) => panic!("failed to run command {args:?}: {error}"),
    }
}

fn output_text(output: &std::process::Output) -> String {
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    combined
}

#[cfg(feature = "lua")]
#[test]
fn cli_should_run_registry_install_use_and_exec_flow() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = temp_workspace();
    let home = temp_dir.path().join("home");
    let project = temp_dir.path().join("project");
    fs::create_dir_all(&project)?;

    let registry_path = fixture_root().join("registry/index.json");
    let registry_path = registry_path.to_string_lossy().into_owned();
    assert_success(run(
        &home,
        &project,
        &["config", "registry.address", registry_path.as_str()],
    ));
    assert_success(run(&home, &project, &["update"]));
    assert_success(run(&home, &project, &["add", "nodejs"]));
    assert_success(run(&home, &project, &["install", "nodejs@20.11.1"]));
    assert_success(run(&home, &project, &["use", "nodejs@20.11.1", "-g"]));

    assert_success(run(&home, &project, &["current", "nodejs"]));
    assert!(output_text(&run(&home, &project, &["current", "nodejs"])).contains("20.11.1"));

    assert_success(run(&home, &project, &["exec", "nodejs", "node"]));
    assert!(
        output_text(&run(&home, &project, &["exec", "nodejs", "node"])).contains("nodejs 20.11.1")
    );
    assert_success(run(&home, &project, &["exec", "nodejs@20.11.1", "node"]));
    assert!(
        output_text(&run(&home, &project, &["exec", "nodejs@20.11.1", "node"]))
            .contains("nodejs 20.11.1")
    );

    assert_success(run(&home, &project, &["uninstall", "nodejs@20.11.1"]));
    assert_success(run(&home, &project, &["remove", "nodejs"]));
    Ok(())
}

#[cfg(feature = "lua")]
#[test]
fn cli_should_prefer_project_scope_and_support_unlink() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = temp_workspace();
    let home = temp_dir.path().join("home");
    let project = temp_dir.path().join("project");
    fs::create_dir_all(&project)?;

    let registry_path = fixture_root().join("registry/index.json");
    let registry_path = registry_path.to_string_lossy().into_owned();
    assert_success(run(
        &home,
        &project,
        &["config", "registry.address", registry_path.as_str()],
    ));
    assert_success(run(&home, &project, &["update"]));
    assert_success(run(&home, &project, &["add", "nodejs"]));
    assert_success(run(&home, &project, &["install", "nodejs@18.19.0"]));
    assert_success(run(&home, &project, &["install", "nodejs@20.11.1"]));
    assert_success(run(&home, &project, &["use", "nodejs@18.19.0", "-g"]));
    assert_success(run(&home, &project, &["use", "nodejs@20.11.1", "-p"]));

    let current = output_text(&run(&home, &project, &["current", "nodejs"]));
    assert!(current.contains("20.11.1"));
    assert!(project.join(".vs.toml").exists());
    assert!(project.join(".vs/sdks/nodejs").exists());

    assert_success(run(&home, &project, &["add", "deno"]));
    assert_success(run(&home, &project, &["install", "deno@1.40.5"]));
    assert_success(run(
        &home,
        &project,
        &["use", "deno@1.40.5", "-p", "--unlink"],
    ));
    assert!(!project.join(".vs/sdks/deno").exists());
    Ok(())
}

#[cfg(feature = "lua")]
#[test]
fn cli_use_without_version_should_require_explicit_version_in_non_interactive_mode()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = temp_workspace();
    let home = temp_dir.path().join("home");
    let project = temp_dir.path().join("project");
    fs::create_dir_all(&project)?;

    let registry_path = fixture_root().join("registry/index.json");
    let registry_path = registry_path.to_string_lossy().into_owned();
    assert_success(run(
        &home,
        &project,
        &["config", "registry.address", registry_path.as_str()],
    ));
    assert_success(run(&home, &project, &["update"]));
    assert_success(run(&home, &project, &["add", "nodejs"]));
    assert_success(run(&home, &project, &["install", "nodejs@20.11.1"]));

    let output = run(&home, &project, &["use", "nodejs"]);
    assert!(!output.status.success());
    assert!(
        output_text(&output)
            .contains("Please specify a version to use in non-interactive environments")
    );
    Ok(())
}

#[cfg(feature = "lua")]
#[test]
fn cli_cd_should_support_home_and_plugin_directories() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = temp_workspace();
    let home = temp_dir.path().join("home");
    let project = temp_dir.path().join("project");
    fs::create_dir_all(&project)?;

    let home_output = run(&home, &project, &["cd"]);
    assert_success(home_output);
    assert_eq!(
        output_text(&run(&home, &project, &["cd"])).trim(),
        home.display().to_string()
    );

    let registry_path = fixture_root().join("registry/index.json");
    let registry_path = registry_path.to_string_lossy().into_owned();
    assert_success(run(
        &home,
        &project,
        &["config", "registry.address", registry_path.as_str()],
    ));
    assert_success(run(&home, &project, &["update"]));
    assert_success(run(&home, &project, &["add", "nodejs"]));

    let plugin_dir = output_text(&run(&home, &project, &["cd", "nodejs", "--plugin"]));
    assert!(Path::new(plugin_dir.trim()).exists());
    Ok(())
}

#[test]
fn cli_should_migrate_from_a_legacy_home() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = temp_workspace();
    let home = temp_dir.path().join("home");
    let project = temp_dir.path().join("project");
    let legacy = temp_dir.path().join("legacy-home");
    fs::create_dir_all(&project)?;
    fs::create_dir_all(legacy.join("global"))?;
    fs::write(
        legacy.join("global/tools.toml"),
        "[tools]\nnodejs = \"18.19.0\"\n",
    )?;

    let legacy_path = legacy.to_string_lossy().into_owned();
    let migrate = run(
        &home,
        &project,
        &["migrate", "--source", legacy_path.as_str()],
    );
    assert_success(migrate);
    assert!(home.join("global/tools.toml").exists());
    Ok(())
}

#[test]
fn cli_help_should_be_english() {
    let output = match Command::new(binary()).arg("--help").output() {
        Ok(output) => output,
        Err(error) => panic!("failed to get help output: {error}"),
    };
    assert_success(output);
    let text = output_text(&match Command::new(binary()).arg("--help").output() {
        Ok(output) => output,
        Err(error) => panic!("failed to get help output: {error}"),
    });
    assert!(text.contains("A runtime version manager inspired by vfox"));
    assert!(text.contains("activate"));
}

fn assert_success(output: std::process::Output) {
    assert!(
        output.status.success(),
        "command failed with status {:?}\n{}",
        output.status.code(),
        output_text(&output)
    );
}
