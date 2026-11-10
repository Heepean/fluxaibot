use std::env;
use std::path::PathBuf;
use std::process::Command;

const RUNTIME_RESOURCES: [(&str, u16); 9] = [
    ("resources/llama-server.exe", 101),
    ("resources/llama-server-vulkan.exe", 102),
    ("resources/llama-server-cuda.exe", 103),
    ("resources/cuda/cublas64_13.dll", 104),
    ("resources/cuda/cublasLt64_13.dll", 105),
    ("resources/cuda/cudart64_13.dll", 106),
    ("resources/cuda/msvcp140.dll", 107),
    ("resources/cuda/vcruntime140.dll", 108),
    ("resources/cuda/vcruntime140_1.dll", 109),
];

fn main() {
    tauri_build::build();

    for (resource, _) in RUNTIME_RESOURCES {
        println!("cargo:rerun-if-changed={resource}");
    }

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_runtime_resources();
    }
}

fn embed_runtime_resources() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is missing"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is missing"));
    let script_path = out_dir.join("fluxaibot-runtime.rc");
    let resource_path = out_dir.join("fluxaibot-runtime.res");

    let mut script = String::new();
    for (resource, id) in RUNTIME_RESOURCES {
        let path = manifest_dir.join(resource);
        if !path.is_file() {
            panic!(
                "Required embedded runtime resource is missing: {}",
                path.display()
            );
        }
        let path = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        script.push_str(&format!("{id} RCDATA \"{path}\"\n"));
    }
    std::fs::write(&script_path, script)
        .unwrap_or_else(|error| panic!("Cannot write runtime resource script: {error}"));

    let compiler = find_resource_compiler();
    let output = Command::new(&compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource_path)
        .arg(&script_path)
        .output()
        .unwrap_or_else(|error| panic!("Cannot run {}: {error}", compiler.display()));
    if !output.status.success() {
        panic!(
            "Windows resource compiler failed ({}):\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("cargo:rustc-link-arg={}", resource_path.display());
}

fn find_resource_compiler() -> PathBuf {
    if let Some(path) = env::var_os("RC")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return path;
    }
    if let Some(paths) = env::var_os("PATH") {
        for directory in env::split_paths(&paths) {
            let path = directory.join("rc.exe");
            if path.is_file() {
                return path;
            }
        }
    }

    let kits = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"))
        .join(r"Windows Kits\10\bin");
    let mut versions = std::fs::read_dir(&kits)
        .unwrap_or_else(|error| {
            panic!(
                "Cannot locate the Windows SDK resource compiler under {}: {error}",
                kits.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("10."))
        })
        .collect::<Vec<_>>();
    versions.sort();
    versions.reverse();

    versions
        .into_iter()
        .map(|version| version.join("x64").join("rc.exe"))
        .find(|path| path.is_file())
        .unwrap_or_else(|| {
            panic!(
                "Windows x64 resource compiler rc.exe was not found. Install the Windows 10/11 SDK."
            )
        })
}
