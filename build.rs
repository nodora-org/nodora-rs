use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

const DEFAULT_BASE_URL: &str = "https://github.com/nodora-org/nodora-rs/releases/download";

fn main() {
    if env::var_os("DOCS_RS").is_some() {
        println!("cargo:warning=DOCS_RS detected; skipping the Nodora native build");
        return;
    }

    println!("cargo:rerun-if-env-changed=GO");
    println!("cargo:rerun-if-env-changed=NODORA_PREBUILT_DIR");
    println!("cargo:rerun-if-env-changed=NODORA_PREBUILT_BASE_URL");
    println!("cargo:rerun-if-env-changed=NODORA_BUILD_FROM_SOURCE");

    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    let bridge_dir = crate_dir.join("bridge");

    let lib = out_dir.join("libnodora.a");

    let lib_dir: PathBuf = if let Some(dir) = env::var_os("NODORA_PREBUILT_DIR") {
        let dir = PathBuf::from(dir);
        assert!(
            dir.join("libnodora.a").exists(),
            "NODORA_PREBUILT_DIR is set but {}/libnodora.a does not exist",
            dir.display()
        );
        dir
    } else if truthy("NODORA_BUILD_FROM_SOURCE") {
        build_from_source(&bridge_dir, &lib);
        out_dir.clone()
    } else if download_prebuilt(&target, &lib) {
        out_dir.clone()
    } else if bridge_dir.join("main.go").exists() && has_go() {
        // universal fallback: no prebuilt for this target
        println!("cargo:warning=no prebuilt archive for {target}; building from source with Go");
        build_from_source(&bridge_dir, &lib);
        out_dir.clone()
    } else {
        panic!(
            "could not obtain a Nodora engine archive for target `{target}`.\n\
             Options:\n  \
             - install Go and build from source (set NODORA_BUILD_FROM_SOURCE=1), or\n  \
             - point NODORA_PREBUILT_DIR at a directory containing libnodora.a, or\n  \
             - use a target with a published prebuilt archive."
        );
    };

    emit_link_flags(&lib_dir, &target);
}

fn emit_link_flags(lib_dir: &Path, target: &str) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=nodora");

    println!("cargo:rustc-link-lib=dylib=pthread");
    if target.contains("linux") {
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=resolv");
    } else if target.contains("apple") || target.contains("darwin") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=Security");
    }
}

fn build_from_source(bridge_dir: &Path, dest: &Path) {
    assert!(
        bridge_dir.join("main.go").exists(),
        "build from source requested but the Go bridge is missing at {}",
        bridge_dir.display()
    );
    println!("cargo:rerun-if-changed={}", bridge_dir.display());

    let go = env::var("GO").unwrap_or_else(|_| "go".to_string());
    let status = Command::new(&go)
        .current_dir(bridge_dir)
        .env("CGO_ENABLED", "1")
        .args([
            "build",
            "-buildmode=c-archive",
            "-o",
            dest.to_str().unwrap(),
            ".",
        ])
        .status()
        .unwrap_or_else(|e| panic!("failed to invoke `{go}`: {e}"));
    assert!(status.success(), "go build (c-archive) failed");
}

fn download_prebuilt(target: &str, dest: &Path) -> bool {
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let base =
        env::var("NODORA_PREBUILT_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let asset = format!("libnodora-{target}.a");
    let url = format!("{base}/v{version}/{asset}");

    if !curl_to_file(&url, dest) {
        return false;
    }

    let sha_text = match curl_to_string(&format!("{url}.sha256")) {
        Some(text) => text,
        None => panic!("downloaded {asset} but its .sha256 checksum is unavailable"),
    };

    let expected = sha_text
        .split_whitespace()
        .next()
        .expect("empty .sha256 file")
        .to_lowercase();

    let actual = sha256_hex(dest);
    assert_eq!(
        actual, expected,
        "checksum mismatch for {asset}: expected {expected}, got {actual}"
    );

    true
}

fn curl_to_file(url: &str, dest: &Path) -> bool {
    Command::new("curl")
        .args(["-fSL", "--retry", "3", "--retry-delay", "2", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn curl_to_string(url: &str) -> Option<String> {
    let out = Command::new("curl").args(["-fsSL", url]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn sha256_hex(path: &Path) -> String {
    let bytes = fs::read(path).expect("read archive for checksum");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn has_go() -> bool {
    let go = env::var("GO").unwrap_or_else(|_| "go".to_string());
    Command::new(go)
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn truthy(var: &str) -> bool {
    matches!(
        env::var(var).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes")
    )
}
