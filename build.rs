extern crate napi_build;

use std::path::Path;
use std::process::Command;

const MECAB_TARBALL: &str = "sources/mecab-0.996.tar.gz";
const MECAB_SRC_PREFIX: &str = "mecab-0.996/src/";
const MECAB_SRC_DIR: &str = ".output/mecab-src";

const CPP_FILES: &[&str] = &[
  "viterbi.cpp",
  "tagger.cpp",
  "utils.cpp",
  "eval.cpp",
  "iconv_utils.cpp",
  "dictionary_rewriter.cpp",
  "dictionary_generator.cpp",
  "dictionary_compiler.cpp",
  "context_id.cpp",
  "connector.cpp",
  "nbest_generator.cpp",
  "writer.cpp",
  "string_buffer.cpp",
  "param.cpp",
  "tokenizer.cpp",
  "char_property.cpp",
  "dictionary.cpp",
  "feature_index.cpp",
  "lbfgs.cpp",
  "learner_tagger.cpp",
  "learner.cpp",
  "libmecab.cpp",
];

/// Extract MeCab C++ sources from the tarball.
fn extract_mecab_src() {
  let mecab_src = Path::new(MECAB_SRC_DIR);
  if mecab_src.exists() {
    return;
  }

  let tarball = Path::new(MECAB_TARBALL);
  assert!(
    tarball.exists(),
    "{} not found — place the MeCab source tarball in the project root",
    MECAB_TARBALL
  );

  std::fs::create_dir_all(mecab_src).expect("failed to create .output/mecab-src/");

  let status = Command::new("tar")
    .args([
      "xzf",
      MECAB_TARBALL,
      "--strip-components=2",
      "-C",
      MECAB_SRC_DIR,
    ])
    .arg(format!("{}*.cpp", MECAB_SRC_PREFIX))
    .arg(format!("{}*.h", MECAB_SRC_PREFIX))
    .status()
    .expect("failed to run tar");
  assert!(status.success(), "tar extraction failed");

  // Remove CLI / tool sources we don't need
  for exclude in &[
    "mecab.cpp",
    "mecab-dict-index.cpp",
    "mecab-dict-gen.cpp",
    "mecab-cost-train.cpp",
    "mecab-system-eval.cpp",
    "mecab-test-gen.cpp",
  ] {
    let p = mecab_src.join(exclude);
    if p.exists() {
      let _ = std::fs::remove_file(p);
    }
  }
}

/// Apply git patches from the patches/ directory.
fn apply_patches() {
  let patches_dir = Path::new("patches");
  if !patches_dir.exists() {
    return;
  }
  let mut entries: Vec<_> = std::fs::read_dir(patches_dir)
    .unwrap()
    .filter_map(|e| e.ok())
    .filter(|e| e.path().extension().map_or(false, |ext| ext == "patch"))
    .collect();
  entries.sort_by_key(|e| e.file_name());
  for entry in entries {
    // Skip if patch is already applied (reverse-apply succeeds)
    let check = Command::new("git")
      .args(["apply", "--check", "--reverse"])
      .arg(entry.path())
      .status();
    if let Ok(s) = check {
      if s.success() {
        continue;
      }
    }
    let status = Command::new("git")
      .args(["apply"])
      .arg(entry.path())
      .status()
      .expect("failed to run git apply");
    assert!(status.success(), "patch failed: {:?}", entry.path());
  }
}

fn main() {
  napi_build::setup();
  extract_mecab_src();
  apply_patches();

  let mecab_src = Path::new(MECAB_SRC_DIR);

  let cpp_files: Vec<_> = CPP_FILES.iter().map(|f| mecab_src.join(f)).collect();

  let mut build = cc::Build::new();
  build
    .cpp(true)
    .include(mecab_src)
    .define("DIC_VERSION", "102")
    .define("HAVE_STDINT_H", None)
    .define("PACKAGE", "\"mecab\"")
    .define("VERSION", "\"0.996\"")
    .define("MECAB_DEFAULT_RC", "\"\"");

  let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
  let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
  let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

  let is_wasm = target_arch == "wasm32";

  if is_wasm {
    build
      .define("HAVE_DIRENT_H", None)
      .define("HAVE_UNISTD_H", None)
      .define("HAVE_SYS_TYPES_H", None)
      .define("HAVE_SYS_STAT_H", None)
      .define("HAVE_FCNTL_H", None)
      .define("HAVE_STRING_H", None)
      .define("HAVE_GETENV", None)
      .define("MECAB_USE_UTF8_ONLY", None)
      .flag("-Wno-unused-variable")
      .flag("-Wno-unused-function")
      .flag("-Wno-unused-parameter")
      .flag("-Wno-register")
      .flag("-Wno-deprecated-register")
      .flag("-Wno-narrowing")
      .flag("-Wno-string-plus-int");
    println!("cargo:rustc-link-lib=static=c++");
    println!("cargo:rustc-link-lib=static=c++abi");
  } else if target_os == "macos" || target_os == "linux" {
    build
      .define("HAVE_DIRENT_H", None)
      .define("HAVE_UNISTD_H", None)
      .define("HAVE_SYS_TYPES_H", None)
      .define("HAVE_SYS_STAT_H", None)
      .define("HAVE_FCNTL_H", None)
      .define("HAVE_SYS_MMAN_H", None)
      .define("HAVE_STRING_H", None)
      .define("HAVE_GETENV", None)
      .define("HAVE_ICONV", None)
      .define("ICONV_CONST", "")
      .flag("-std=c++11")
      .flag("-Wno-unused-variable")
      .flag("-Wno-unused-function")
      .flag("-Wno-unused-parameter");

    if target_os == "macos" {
      build
        .flag("-Wno-c++11-narrowing")
        .flag("-Wno-deprecated-register")
        .flag("-Wno-sometimes-uninitialized");
      println!("cargo:rustc-link-lib=c++");
      println!("cargo:rustc-link-lib=iconv");
    } else {
      build.flag("-Wno-narrowing");
      println!("cargo:rustc-link-lib=stdc++");
    }
  } else if target_os == "windows" {
    build
      .define("_CRT_SECURE_NO_WARNINGS", None)
      .define("NOMINMAX", None)
      .flag("/EHsc")
      .flag("/W0");
  }

  if target_env == "msvc" {
    build.flag("/utf-8");
  }

  for f in &cpp_files {
    build.file(f);
  }

  build.compile("mecab");

  println!("cargo:rerun-if-changed={}", MECAB_TARBALL);
  println!("cargo:rerun-if-changed=patches");
}
