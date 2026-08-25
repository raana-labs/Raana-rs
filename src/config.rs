use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

pub const LOCAL_DDK_IMAGE_PREFIX: &str = "docker.cnb.cool/ylarod/ddk/ddk-min:";
pub const RUST_IMAGE_TARGET: &str = "android16-6.12";
pub const RUSTC_PATH: &str = "/opt/ddk/rust/rust-1.82.0/bin/rustc";

pub const DEFAULT_KMSDK_REV: &str = "ddda0b34b9f89f784e3c92f256a00a27a5198d42";
pub const DEFAULT_RUST_SUPPORT_REV: &str = "dc55b5e7b25df374a8d446a5f9e4c73d3e818a38";

pub const DEFAULT_CACHE_DIR: &str = ".cache";
pub const DEFAULT_TARGET: &str = "android16-6.12";
pub const OBJCOPY_CHUNK: usize = 500;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub image_prefix: String,
    pub rust_image_target: String,
    pub rustc_path: String,
    pub kmsdk_rev: String,
    pub rust_support_rev: String,
    pub cache_dir: String,
    pub objcopy_chunk: usize,
}

impl RuntimeConfig {
    pub fn from_env() -> RuntimeConfig {
        RuntimeConfig {
            image_prefix: env_or("RAANA_IMAGE_PREFIX", LOCAL_DDK_IMAGE_PREFIX),
            rust_image_target: env_or("RAANA_RUST_IMAGE_TARGET", RUST_IMAGE_TARGET),
            rustc_path: env_or("RAANA_RUSTC_PATH", RUSTC_PATH),
            kmsdk_rev: env_or("RAANA_KMSDK_REV", DEFAULT_KMSDK_REV),
            rust_support_rev: env_or("RAANA_RUST_SUPPORT_REV", DEFAULT_RUST_SUPPORT_REV),
            cache_dir: env_or("RAANA_CACHE_DIR", DEFAULT_CACHE_DIR),
            objcopy_chunk: std::env::var("RAANA_OBJCOPY_CHUNK")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(OBJCOPY_CHUNK),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScaffoldConfig {
    pub author: String,
    pub email: String,
    pub year: String,
    pub license_spdx: String,
    pub header_c: String,
    pub header_rust: String,
}

impl ScaffoldConfig {
    pub fn from_env() -> ScaffoldConfig {
        let default_c = format!(
            "// SPDX-License-Identifier: {{spdx}}\n/*\n * Copyright (C) {{year}} {{author}}\n */\n"
        );
        let default_rust = default_c.clone();

        ScaffoldConfig {
            author: env_or("RAANA_AUTHOR", "dere3046"),
            email: env_or("RAANA_EMAIL", ""),
            year: env_or("RAANA_YEAR", &current_year()),
            license_spdx: env_or("RAANA_LICENSE_SPDX", "GPL-2.0-only"),
            header_c: read_template_env("RAANA_HEADER_C_FILE", "RAANA_HEADER_FILE", &default_c),
            header_rust: read_template_env(
                "RAANA_HEADER_RUST_FILE",
                "RAANA_HEADER_FILE",
                &default_rust,
            ),
        }
    }

    pub fn apply_user_config(&mut self) {
        let Some(config) = load_user_toml() else {
            return;
        };

        if let Some(author) = config.author.as_ref().and_then(|a| a.name.clone()) {
            self.author = author;
        }
        if let Some(email) = config.author.as_ref().and_then(|a| a.email.clone()) {
            self.email = email;
        }
        if let Some(spdx) = config.license.as_ref().and_then(|l| l.spdx.clone()) {
            self.license_spdx = spdx;
        }
        if let Some(header) = config.license.as_ref().and_then(|l| l.header_c.clone()) {
            self.header_c = header;
        } else if let Some(header) = config.license.as_ref().and_then(|l| l.header.clone()) {
            self.header_c = header;
        }
        if let Some(header) = config.license.as_ref().and_then(|l| l.header_rust.clone()) {
            self.header_rust = header;
        } else if let Some(header) = config.license.as_ref().and_then(|l| l.header.clone()) {
            self.header_rust = header;
        }
    }

    pub fn render_c(&self, name: &str) -> String {
        render_template(&self.header_c, name, self)
    }

    pub fn render_rust(&self, name: &str) -> String {
        render_template(&self.header_rust, name, self)
    }
}

fn render_template(template: &str, name: &str, cfg: &ScaffoldConfig) -> String {
    template
        .replace("{author}", &cfg.author)
        .replace("{email}", &cfg.email)
        .replace("{year}", &cfg.year)
        .replace("{spdx}", &cfg.license_spdx)
        .replace("{name}", name)
}

fn read_template_env(primary: &str, shared: &str, default: &str) -> String {
    if let Ok(path) = std::env::var(primary) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return content;
        }
    }
    if let Ok(path) = std::env::var(shared) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return content;
        }
    }
    default.to_string()
}

fn current_year() -> String {
    if let Ok(output) = Command::new("date").arg("+%Y").output() {
        if output.status.success() {
            if let Ok(year) = String::from_utf8(output.stdout) {
                let year = year.trim();
                if !year.is_empty() {
                    return year.to_string();
                }
            }
        }
    }
    "2026".to_string()
}

#[derive(Debug, Deserialize)]
struct UserToml {
    #[serde(default)]
    author: Option<UserAuthor>,
    #[serde(default)]
    license: Option<UserLicense>,
    #[serde(default)]
    sdk: Option<UserSdk>,
}

#[derive(Debug, Deserialize)]
struct UserSdk {
    #[serde(default)]
    prebuilt: Option<bool>,
}

pub fn prebuilt_effective(manifest_prebuilt: bool) -> bool {
    if let Ok(v) = std::env::var("RAANA_PREBUILT") {
        return v == "1" || v.eq_ignore_ascii_case("true");
    }
    if let Some(config) = load_user_toml() {
        if let Some(sdk) = config.sdk {
            if let Some(prebuilt) = sdk.prebuilt {
                return prebuilt;
            }
        }
    }
    manifest_prebuilt
}

#[derive(Debug, Deserialize)]
struct UserAuthor {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserLicense {
    #[serde(default)]
    spdx: Option<String>,
    #[serde(default)]
    header: Option<String>,
    #[serde(default, rename = "header-c")]
    header_c: Option<String>,
    #[serde(default, rename = "header-rust")]
    header_rust: Option<String>,
}

fn load_user_toml() -> Option<UserToml> {
    let home = std::env::var("HOME").ok()?;
    let path = PathBuf::from(home).join(".raana.toml");
    let content = std::fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
