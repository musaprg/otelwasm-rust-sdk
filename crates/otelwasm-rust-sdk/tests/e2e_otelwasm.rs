use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate path has no parent")
        .parent()
        .expect("crate path has no grand-parent")
        .to_path_buf()
}

fn sibling_otelwasm_root(workspace_root: &Path) -> PathBuf {
    workspace_root
        .parent()
        .expect("workspace has no parent")
        .join("otelwasm")
}

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn e2e_otelwasm_can_run_rust_guest() {
    let workspace_root = workspace_root();
    let otelwasm_root = sibling_otelwasm_root(&workspace_root);
    if !otelwasm_root.join("wasmplugin").exists() {
        eprintln!(
            "skipping e2e_otelwasm_can_run_rust_guest: expected sibling otelwasm checkout at {}",
            otelwasm_root.display()
        );
        return;
    }
    if !go_available() {
        eprintln!("skipping e2e_otelwasm_can_run_rust_guest: `go` is not available");
        return;
    }

    build_wasm_package(&workspace_root, "otelwasm-rust-example-traces-processor");
    build_wasm_package(&workspace_root, "otelwasm-rust-example-traces-exporter");
    build_wasm_package(&workspace_root, "otelwasm-rust-example-traces-receiver");

    let processor_wasm_path = wasm_path(
        &workspace_root,
        "otelwasm_rust_example_traces_processor.wasm",
    );
    let exporter_wasm_path = wasm_path(
        &workspace_root,
        "otelwasm_rust_example_traces_exporter.wasm",
    );
    let receiver_wasm_path = wasm_path(
        &workspace_root,
        "otelwasm_rust_example_traces_receiver.wasm",
    );
    for path in [
        &processor_wasm_path,
        &exporter_wasm_path,
        &receiver_wasm_path,
    ] {
        assert!(
            path.exists(),
            "expected guest wasm binary to exist at {}",
            path.display()
        );
    }

    let go_harness_dir = workspace_root.join("e2e").join("go_harness");
    let go_cache_dir = workspace_root.join("target").join("go-build-cache");
    let test_status = Command::new("go")
        .arg("test")
        .arg("./...")
        .env("OTELWASM_PROCESSOR_WASM_PATH", &processor_wasm_path)
        .env("OTELWASM_EXPORTER_WASM_PATH", &exporter_wasm_path)
        .env("OTELWASM_RECEIVER_WASM_PATH", &receiver_wasm_path)
        .env("GOCACHE", &go_cache_dir)
        .current_dir(&go_harness_dir)
        .status()
        .expect("failed to execute go test");
    assert!(
        test_status.success(),
        "go e2e test failed with status: {test_status}"
    );
}

fn build_wasm_package(workspace_root: &Path, package: &str) {
    let build_status = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg(package)
        .arg("--target")
        .arg("wasm32-wasip1")
        .current_dir(workspace_root)
        .status()
        .expect("failed to execute cargo build");
    assert!(
        build_status.success(),
        "building wasm guest package {package} failed with status: {build_status}"
    );
}

fn wasm_path(workspace_root: &Path, filename: &str) -> PathBuf {
    workspace_root
        .join("target")
        .join("wasm32-wasip1")
        .join("debug")
        .join(filename)
}

#[test]
fn e2e_artifacts_exist() {
    let go_harness_test = workspace_root()
        .join("e2e")
        .join("go_harness")
        .join("e2e_test.go");
    assert!(
        Path::new(&go_harness_test).exists(),
        "missing go e2e test at {}",
        go_harness_test.display()
    );
}
