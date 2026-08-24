use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{config, GkiTarget, Sdk};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: Package,
    pub sdk: SdkConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub dependencies: Dependencies,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_rust")]
    pub rust: String,
    #[serde(default = "default_wrapper")]
    pub wrapper: String,
    #[serde(default)]
    pub kunit: bool,
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkConfig {
    #[serde(default = "default_kmsdk")]
    pub kmsdk: String,
    #[serde(default = "default_rust_support", rename = "rust-support")]
    pub rust_support: String,
    #[serde(default, rename = "runtime-dir")]
    pub runtime_dir: Option<String>,
    #[serde(default, rename = "image-prefix")]
    pub image_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default = "default_cache")]
    pub cache: String,
    #[serde(default)]
    pub runner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dependencies {
    #[serde(default)]
    pub c: BTreeMap<String, CDependency>,
    #[serde(default)]
    pub kmsdk: KmsdkDeps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDependency {
    pub path: String,
    #[serde(default)]
    pub objs: Vec<String>,
    #[serde(default)]
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KmsdkDeps {
    #[serde(default)]
    pub libs: Vec<String>,
}

fn default_language() -> String {
    "rust".to_string()
}

fn default_rust() -> String {
    "src/lib.rs".to_string()
}

fn default_wrapper() -> String {
    "src/wrapper.c".to_string()
}

fn default_kmsdk() -> String {
    config::DEFAULT_KMSDK_REV.to_string()
}

fn default_rust_support() -> String {
    config::DEFAULT_RUST_SUPPORT_REV.to_string()
}

fn default_cache() -> String {
    config::DEFAULT_CACHE_DIR.to_string()
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn sdk(&self) -> Sdk {
        Sdk {
            kmsdk_rev: self.sdk.kmsdk.clone(),
            rust_support_rev: self.sdk.rust_support.clone(),
        }
    }

    pub fn targets(&self) -> Vec<GkiTarget> {
        self.build
            .targets
            .iter()
            .filter_map(|name| GkiTarget::from_name(name))
            .collect()
    }
}
