use heck::ToSnakeCase;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use wit_component::ComponentEncoder;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=../../src");
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .current_dir("../../")
        .arg("--target=wasm32-unknown-unknown")
        .arg("--features=command")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let command_adapter =
        out_dir.join("wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm");
    println!("wasi command adapter: {:?}", &command_adapter);
    let command_adapter = fs::read(&command_adapter).unwrap();

    println!("cargo:rerun-if-changed=../../src");
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .current_dir("../../")
        .arg("--target=wasm32-unknown-unknown")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let reactor_adapter =
        out_dir.join("wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm");
    println!("wasi reactor adapter: {:?}", &reactor_adapter);
    let reactor_adapter = fs::read(&reactor_adapter).unwrap();

    // Build all test program crates:
    println!("cargo:rerun-if-changed=..");
    let mut cmd = Command::new("rustup");
    cmd.arg("run")
        .arg("nightly")
        .arg("cargo")
        .arg("build")
        .arg("--target=wasm32-wasi")
        .arg("--package=wasi-tests")
        .arg("--package=test-programs")
        .arg("--package=reactor-tests")
        .current_dir("..")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "1")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let meta = cargo_metadata::MetadataCommand::new().exec().unwrap();

    let mut command_components = Vec::new();

    for stem in targets_in_package(&meta, "test-programs", "bin").chain(targets_in_package(
        &meta,
        "wasi-tests",
        "bin",
    )) {
        let file = out_dir
            .join("wasm32-wasi/debug")
            .join(format!("{stem}.wasm"));

        let module = fs::read(&file).expect("read wasm module");
        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .unwrap()
            .validate(true)
            .adapter("wasi_snapshot_preview1", &command_adapter)
            .unwrap()
            .encode()
            .expect(&format!(
                "module {:?} can be translated to a component",
                file
            ));
        let component_path = out_dir.join(format!("{}.component.wasm", &stem));
        fs::write(&component_path, component).expect("write component to disk");
        command_components.push((stem, component_path));
    }

    let mut reactor_components = Vec::new();

    for stem in targets_in_package(&meta, "reactor-tests", "cdylib") {
        let stem = stem.to_snake_case();
        let file = out_dir
            .join("wasm32-wasi/debug")
            .join(format!("{stem}.wasm"));

        let module = fs::read(&file).expect(&format!("read wasm module: {file:?}"));
        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .unwrap()
            .validate(true)
            .adapter("wasi_snapshot_preview1", &reactor_adapter)
            .unwrap()
            .encode()
            .expect(&format!(
                "module {:?} can be translated to a component",
                file
            ));
        let component_path = out_dir.join(format!("{}.component.wasm", &stem));
        fs::write(&component_path, component).expect("write component to disk");
        reactor_components.push((stem, component_path));
    }

    let src = format!(
        "const COMMAND_COMPONENTS: &[(&str, &str)] = &{command_components:?};
         const REACTOR_COMPONENTS: &[(&str, &str)] = &{reactor_components:?};
        ",
    );
    std::fs::write(out_dir.join("components.rs"), src).unwrap();
}

fn targets_in_package<'a>(
    meta: &'a cargo_metadata::Metadata,
    package: &'a str,
    kind: &'a str,
) -> impl Iterator<Item = &'a String> + 'a {
    meta.packages
        .iter()
        .find(|p| p.name == package)
        .unwrap()
        .targets
        .iter()
        .filter(move |t| t.kind == &[kind])
        .map(|t| &t.name)
}
