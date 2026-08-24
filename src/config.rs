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

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
