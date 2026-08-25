use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod config;
pub mod manifest;
pub mod scaffold;

use manifest::Manifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GkiTarget {
    Android12_5_10,
    Android13_5_10,
    Android13_5_15,
    Android14_5_15,
    Android14_6_1,
    Android15_6_6,
    Android16_6_12,
}

impl GkiTarget {
    pub fn all() -> &'static [GkiTarget] {
        &[
            GkiTarget::Android12_5_10,
            GkiTarget::Android13_5_10,
            GkiTarget::Android13_5_15,
            GkiTarget::Android14_5_15,
            GkiTarget::Android14_6_1,
            GkiTarget::Android15_6_6,
            GkiTarget::Android16_6_12,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            GkiTarget::Android12_5_10 => "android12-5.10",
            GkiTarget::Android13_5_10 => "android13-5.10",
            GkiTarget::Android13_5_15 => "android13-5.15",
            GkiTarget::Android14_5_15 => "android14-5.15",
            GkiTarget::Android14_6_1 => "android14-6.1",
            GkiTarget::Android15_6_6 => "android15-6.6",
            GkiTarget::Android16_6_12 => "android16-6.12",
        }
    }

    pub fn from_name(name: &str) -> Option<GkiTarget> {
        GkiTarget::all().iter().copied().find(|t| t.name() == name)
    }

    pub fn ddk_image(self) -> String {
        format!("{}{}", config::LOCAL_DDK_IMAGE_PREFIX, self.name())
    }

    pub fn rust_image(self) -> String {
        format!(
            "{}{}",
            config::LOCAL_DDK_IMAGE_PREFIX,
            config::RUST_IMAGE_TARGET
        )
    }

    pub fn kdir(self) -> String {
        format!("/opt/ddk/kdir/{}", self.name())
    }

    pub fn short_alias(self) -> bool {
        matches!(
            self,
            GkiTarget::Android12_5_10
                | GkiTarget::Android13_5_10
                | GkiTarget::Android13_5_15
                | GkiTarget::Android14_5_15
        )
    }

    pub fn skip_rust_fmt(self) -> bool {
        matches!(self, GkiTarget::Android16_6_12)
    }

    pub fn skip_rust_helpers(self) -> bool {
        matches!(self, GkiTarget::Android16_6_12)
    }

    pub fn modpost_fix(self) -> bool {
        matches!(
            self,
            GkiTarget::Android14_6_1 | GkiTarget::Android15_6_6 | GkiTarget::Android16_6_12
        )
    }
}

#[derive(Debug, Clone)]
pub struct Sdk {
    pub kmsdk_rev: String,
    pub rust_support_rev: String,
}

impl Sdk {
    pub fn current() -> Sdk {
        let cfg = config::RuntimeConfig::from_env();
        Sdk {
            kmsdk_rev: cfg.kmsdk_rev,
            rust_support_rev: cfg.rust_support_rev,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildPaths {
    pub project_root: PathBuf,
    pub cache_root: PathBuf,
    pub target: GkiTarget,
    pub rust_support_rev: String,
    pub runtime_dir: Option<PathBuf>,
    pub runtime: config::RuntimeConfig,
    pub use_container_paths: bool,
}

impl BuildPaths {
    pub fn new(
        project_root: PathBuf,
        cache_root: PathBuf,
        target: GkiTarget,
        rust_support_rev: String,
        runtime_dir: Option<PathBuf>,
        runtime: config::RuntimeConfig,
        use_container_paths: bool,
    ) -> BuildPaths {
        BuildPaths {
            project_root,
            cache_root,
            target,
            rust_support_rev,
            runtime_dir,
            runtime,
            use_container_paths,
        }
    }

    pub fn ddk_image(&self) -> String {
        format!("{}{}", self.runtime.image_prefix, self.target.name())
    }

    pub fn rust_image(&self) -> String {
        format!(
            "{}{}",
            self.runtime.image_prefix, self.runtime.rust_image_target
        )
    }

    pub fn project_path(&self) -> String {
        if self.use_container_paths {
            "/src".to_string()
        } else {
            self.project_root.to_string_lossy().to_string()
        }
    }

    pub fn cache_path(&self) -> String {
        if self.use_container_paths {
            "/cache".to_string()
        } else {
            self.cache_root.to_string_lossy().to_string()
        }
    }

    pub fn container_project(&self) -> &str {
        "/src"
    }

    pub fn container_cache(&self) -> &str {
        "/cache"
    }

    pub fn host_rust_support_dir(&self) -> PathBuf {
        if let Some(dir) = &self.runtime_dir {
            let out_target = dir.join("out").join(self.target.name());
            if out_target.join("rust_support.ko").exists() {
                return out_target;
            }
            if dir.join("rust_support.ko").exists() {
                return dir.clone();
            }
        }
        self.cache_root
            .join("rust_support")
            .join(&self.rust_support_rev)
            .join(self.target.name())
    }

    pub fn container_rust_support_dir(&self) -> String {
        if let Some(dir) = &self.runtime_dir {
            let out_target = dir.join("out").join(self.target.name());
            if out_target.join("rust_support.ko").exists() {
                return format!("/runtime/out/{}", self.target.name());
            }
            if dir.join("rust_support.ko").exists() {
                return "/runtime".to_string();
            }
        }
        format!(
            "/cache/rust_support/{}/{}",
            self.rust_support_rev,
            self.target.name()
        )
    }

    pub fn command_rust_support_dir(&self) -> String {
        if self.use_container_paths {
            self.container_rust_support_dir()
        } else {
            self.host_rust_support_dir().to_string_lossy().to_string()
        }
    }

    pub fn host_out_dir(&self) -> PathBuf {
        self.project_root.join("out").join(self.target.name())
    }

    pub fn container_out_dir(&self) -> String {
        format!("/src/out/{}", self.target.name())
    }

    pub fn command_out_dir(&self) -> String {
        if self.use_container_paths {
            self.container_out_dir()
        } else {
            self.host_out_dir().to_string_lossy().to_string()
        }
    }

    pub fn host_rust_dir(&self) -> PathBuf {
        self.host_rust_support_dir().join("rust")
    }

    pub fn container_rust_dir(&self) -> String {
        format!("{}/rust", self.container_rust_support_dir())
    }

    pub fn command_rust_dir(&self) -> String {
        if self.use_container_paths {
            self.container_rust_dir()
        } else {
            self.host_rust_dir().to_string_lossy().to_string()
        }
    }

    pub fn host_sym_map(&self) -> PathBuf {
        self.host_rust_support_dir().join("rust_sym_map.txt")
    }

    pub fn mounts(&self) -> Vec<(String, String)> {
        if !self.use_container_paths {
            return Vec::new();
        }
        let mut mounts = vec![
            (
                self.project_root.to_string_lossy().to_string(),
                self.container_project().to_string(),
            ),
            (
                self.cache_root.to_string_lossy().to_string(),
                self.container_cache().to_string(),
            ),
        ];
        if let Some(dir) = &self.runtime_dir {
            mounts.push((dir.to_string_lossy().to_string(), "/runtime".to_string()));
        }
        mounts
    }
}

pub enum RunnerMode {
    Docker { image: String },
    Native,
}

pub struct Runner {
    pub mode: RunnerMode,
}

impl Runner {
    pub fn detect(image: &str) -> Runner {
        Runner::detect_with_force(image, false)
    }

    pub fn detect_with_force(image: &str, force_docker: bool) -> Runner {
        if !force_docker && Path::new("/opt/ddk").exists() {
            Runner {
                mode: RunnerMode::Native,
            }
        } else {
            Runner {
                mode: RunnerMode::Docker {
                    image: image.to_string(),
                },
            }
        }
    }

    pub fn run(&self, args: &[&str]) -> Result<(), String> {
        self.run_in("", &[], &[], args)
    }

    pub fn run_in(
        &self,
        workdir: &str,
        mounts: &[(String, String)],
        envs: &[(&str, &str)],
        args: &[&str],
    ) -> Result<(), String> {
        let status = match &self.mode {
            RunnerMode::Native => {
                let mut cmd = Command::new(args[0]);
                cmd.args(&args[1..]);
                for (key, value) in envs {
                    cmd.env(key, value);
                }
                if !workdir.is_empty() {
                    cmd.current_dir(workdir);
                }
                cmd.status().map_err(|e| e.to_string())?
            }
            RunnerMode::Docker { image } => {
                let mut cmd = Command::new("docker");
                cmd.arg("run").arg("--rm");
                for (host, container) in mounts {
                    cmd.arg("-v").arg(format!("{}:{}", host, container));
                }
                for (key, value) in envs {
                    cmd.arg("-e").arg(format!("{}={}", key, value));
                }
                if !workdir.is_empty() {
                    cmd.arg("-w").arg(workdir);
                }
                cmd.arg(image);
                cmd.args(args);
                cmd.status().map_err(|e| e.to_string())?
            }
        };

        if status.success() {
            Ok(())
        } else {
            Err(format!("command failed with {}", status))
        }
    }
}

pub fn has_ddk_host(target: GkiTarget, cfg: &config::RuntimeConfig) -> bool {
    Path::new("/opt/ddk/kdir").join(target.name()).exists() && Path::new(&cfg.rustc_path).exists()
}

pub fn fetch_project(project_root: &Path, manifest: &Manifest) -> Result<(), String> {
    let sdk_dir = project_root.join(".sdk");
    if !sdk_dir.join(".git").exists() {
        run_cmd(
            "git",
            &[
                "clone",
                "https://github.com/Dere3046/KMSDK.git",
                sdk_dir.to_str().ok_or("bad path")?,
            ],
            project_root,
        )?;
    }

    run_cmd(
        "git",
        &[
            "-C",
            sdk_dir.to_str().ok_or("bad path")?,
            "checkout",
            &manifest.sdk.kmsdk,
        ],
        project_root,
    )?;
    let deps_lst = project_root.join("deps.lst");
    let mut deps_content = std::fs::read_to_string(&deps_lst).unwrap_or_default();
    if deps_content.is_empty() {
        deps_content.push_str("# <name> <rev>\n");
    }

    if manifest.package.language != "c" {
        let has_rust_support = deps_content
            .lines()
            .any(|line| line.starts_with("rust_support "));
        if !has_rust_support {
            deps_content.push_str(&format!("rust_support {}\n", manifest.sdk.rust_support));
        }
    }

    for lib in &manifest.dependencies.kmsdk.libs {
        let prefix = format!("{} ", lib);
        let has = deps_content.lines().any(|line| line.starts_with(&prefix));
        if !has {
            deps_content.push_str(&format!("{}\n", lib));
        }
    }

    std::fs::write(&deps_lst, deps_content).map_err(|e| e.to_string())?;

    run_cmd(
        sdk_dir.join("scripts/sdk").to_str().ok_or("bad path")?,
        &["install"],
        project_root,
    )?;

    let rs_dir = project_root.join("deps/rust_support");
    if rs_dir.join("scripts/fetch-deps.sh").exists() {
        run_cmd("sh", &["scripts/fetch-deps.sh"], &rs_dir)?;
    }

    Ok(())
}

fn run_cmd(program: &str, args: &[&str], cwd: &Path) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed with {}", program, status))
    }
}

pub fn compile_module(
    paths: &BuildPaths,
    module_name: &str,
    rust_src_rel: &str,
    kunit: bool,
) -> Result<PathBuf, String> {
    let out_dir = paths.host_out_dir();
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let mut args = vec![
        paths.runtime.rustc_path.clone(),
        "--edition=2021".to_string(),
        "-Cpanic=abort".to_string(),
        "-Cembed-bitcode=n".to_string(),
        "-Clto=n".to_string(),
        "-Ccodegen-units=1".to_string(),
        "-Csymbol-mangling-version=v0".to_string(),
        "-Crelocation-model=static".to_string(),
        "--target=aarch64-unknown-none".to_string(),
        "-Ctarget-feature=-neon".to_string(),
        "--crate-type".to_string(),
        "rlib".to_string(),
        "--crate-name".to_string(),
        module_name.to_string(),
        "-L".to_string(),
        paths.command_rust_dir(),
        "--extern".to_string(),
        "kernel".to_string(),
        "--extern".to_string(),
        "core".to_string(),
        "--extern".to_string(),
        "compiler_builtins".to_string(),
        "--extern".to_string(),
        "macros".to_string(),
        "--cfg".to_string(),
        "MODULE".to_string(),
    ];
    if kunit {
        args.push("--cfg".to_string());
        args.push("CONFIG_KUNIT".to_string());
    }
    args.push(format!(
        "--emit=obj={}/{}_rust.o",
        paths.command_out_dir(),
        module_name
    ));
    args.push("--sysroot=/dev/null".to_string());
    args.push(format!("{}/{}", paths.project_path(), rust_src_rel));

    let runner = Runner::detect_with_force(&paths.rust_image(), paths.use_container_paths);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let envs = [("RUST_MODFILE", module_name)];
    runner.run_in(&paths.project_path(), &paths.mounts(), &envs, &arg_refs)?;

    let obj = out_dir.join(format!("{}_rust.o", module_name));
    rename_entry_points(&obj, module_name)?;
    apply_aliases_with_chunk(&obj, &paths.host_sym_map(), paths.runtime.objcopy_chunk)?;

    let cmd_path = out_dir.join(format!(".{}_rust.o.cmd", module_name));
    if !cmd_path.exists() {
        std::fs::write(&cmd_path, "").map_err(|e| e.to_string())?;
    }

    Ok(obj)
}

pub fn rename_entry_points(obj: &Path, module_name: &str) -> Result<(), String> {
    let base = module_name.strip_suffix("_rust").unwrap_or(module_name);
    let init = format!("rust_{}_init_module", base);
    let cleanup = format!("rust_{}_cleanup_module", base);

    let status = Command::new("llvm-objcopy")
        .arg("--redefine-sym")
        .arg(format!("init_module={}", init))
        .arg("--redefine-sym")
        .arg(format!("cleanup_module={}", cleanup))
        .arg(obj)
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("rename_entry_points failed with {}", status))
    }
}

pub fn apply_aliases(obj: &Path, map: &Path) -> Result<(), String> {
    let chunk_size = config::RuntimeConfig::from_env().objcopy_chunk;
    apply_aliases_with_chunk(obj, map, chunk_size)
}

pub fn apply_aliases_with_chunk(obj: &Path, map: &Path, chunk_size: usize) -> Result<(), String> {
    if !map.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(map).map_err(|e| e.to_string())?;
    let pairs: Vec<(String, String)> = content
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let long = parts.next()?;
            let short = parts.next()?;
            Some((long.to_string(), short.to_string()))
        })
        .collect();

    for chunk in pairs.chunks(chunk_size) {
        let mut cmd = Command::new("llvm-objcopy");
        for (long, short) in chunk {
            cmd.arg("--redefine-sym").arg(format!("{}={}", long, short));
        }
        cmd.arg(obj);
        let status = cmd.status().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("apply_aliases failed with {}", status));
        }
    }

    Ok(())
}

pub fn link_module(paths: &BuildPaths, module_name: &str) -> Result<PathBuf, String> {
    let symvers = paths.command_rust_support_dir();
    link_module_with_symvers(paths, module_name, Some(&symvers))
}

pub fn link_module_with_symvers(
    paths: &BuildPaths,
    module_name: &str,
    extra_symvers: Option<&str>,
) -> Result<PathBuf, String> {
    let mut args = vec![
        "make".to_string(),
        format!("KDIR={}", paths.target.kdir()),
        format!("VER={}", paths.target.name()),
    ];
    if let Some(symvers) = extra_symvers {
        args.push(format!("KBUILD_EXTRA_SYMBOLS={}/Module.symvers", symvers));
    }

    let runner = Runner::detect_with_force(&paths.ddk_image(), paths.use_container_paths);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    runner.run_in(&paths.project_path(), &paths.mounts(), &[], &arg_refs)?;

    Ok(paths.host_out_dir().join(format!("{}.ko", module_name)))
}

pub fn build_from_manifest(
    manifest: &Manifest,
    project_root: &Path,
    target: GkiTarget,
) -> Result<PathBuf, String> {
    let project_root = project_root.canonicalize().map_err(|e| e.to_string())?;
    let cache_root = project_root.join(&manifest.build.cache);
    let runtime_dir = manifest.sdk.runtime_dir.as_ref().map(|d| {
        PathBuf::from(d)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(d))
    });
    let mut runtime = config::RuntimeConfig::from_env();
    if let Some(prefix) = &manifest.sdk.image_prefix {
        runtime.image_prefix = prefix.clone();
    }
    runtime.kmsdk_rev = manifest.sdk.kmsdk.clone();
    runtime.rust_support_rev = manifest.sdk.rust_support.clone();
    if let Some(path) = &manifest.sdk.rustc_path {
        runtime.rustc_path = path.clone();
    }
    if let Some(target) = &manifest.sdk.rust_image_target {
        runtime.rust_image_target = target.clone();
    }
    if let Some(chunk) = manifest.build.objcopy_chunk {
        runtime.objcopy_chunk = chunk;
    }

    let use_container_paths = match manifest.build.runner.as_deref() {
        Some("docker") => true,
        Some("host") => false,
        _ => !has_ddk_host(target, &runtime),
    };
    if !use_container_paths && !has_ddk_host(target, &runtime) {
        return Err("host mode requires /opt/ddk with kdir and rust toolchain".to_string());
    }
    let paths = BuildPaths::new(
        project_root.clone(),
        cache_root,
        target,
        manifest.sdk.rust_support.clone(),
        runtime_dir,
        runtime,
        use_container_paths,
    );

    let ko = paths
        .host_out_dir()
        .join(format!("{}.ko", manifest.package.name));
    let stamp_dir = project_root.join(".raana_cache");
    let stamp = stamp_dir.join(format!("{}.stamp", target.name()));
    let hash = build_hash(manifest, target);

    if ko.exists() && stamp.exists() {
        let old = std::fs::read_to_string(&stamp).map_err(|e| e.to_string())?;
        if old == hash {
            return Ok(ko);
        }
    }

    sync_makefile(manifest, &project_root)?;

    if manifest.package.language == "c" {
        link_module_with_symvers(&paths, &manifest.package.name, None)?;
    } else {
        ensure_rust_support(&paths, manifest)?;
        compile_module(
            &paths,
            &manifest.package.name,
            &manifest.package.rust,
            manifest.package.kunit,
        )?;
        link_module(&paths, &manifest.package.name)?;
    }

    std::fs::create_dir_all(&stamp_dir).map_err(|e| e.to_string())?;
    std::fs::write(&stamp, hash).map_err(|e| e.to_string())?;

    Ok(ko)
}

pub fn ensure_rust_support(paths: &BuildPaths, manifest: &Manifest) -> Result<(), String> {
    let dir = paths.host_rust_support_dir();
    if rust_support_cache_valid(paths) {
        return Ok(());
    }

    if paths.runtime_dir.is_some() {
        return Err(format!(
            "rust_support artifacts missing for {} at {}\ncheck runtime-dir",
            paths.target.name(),
            dir.display()
        ));
    }

    if config::prebuilt_effective(manifest.sdk.prebuilt) {
        fetch_prebuilt(paths, manifest)?;
        if !rust_support_cache_valid(paths) {
            return Err(format!("prebuilt cache incomplete at {}", dir.display()));
        }
        return Ok(());
    }

    let source = paths.project_root.join("deps/rust_support");
    if source.join("scripts/build-ddkk.sh").exists() {
        run_cmd(
            "sh",
            &[
                source
                    .join("scripts/build-ddkk.sh")
                    .to_str()
                    .ok_or("bad path")?,
                paths.target.name(),
            ],
            &source,
        )?;

        let out_target = source.join("out").join(paths.target.name());
        if out_target.join("rust_support.ko").exists() {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let copy_src = format!("{}/.", out_target.display());
            run_cmd(
                "cp",
                &["-a", copy_src.as_str(), dir.to_str().ok_or("bad path")?],
                &paths.project_root,
            )?;
            if !rust_support_cache_valid(paths) {
                return Err(format!("local build cache incomplete at {}", dir.display()));
            }
            return Ok(());
        }
    }

    Err(format!(
        "rust_support artifacts missing for {} at {}\nset [sdk] runtime-dir, prebuilt, or run `raana fetch`",
        paths.target.name(),
        dir.display()
    ))
}

fn rust_support_cache_valid(paths: &BuildPaths) -> bool {
    let dir = paths.host_rust_support_dir();
    let required = [
        dir.join("rust_support.ko"),
        dir.join("Module.symvers"),
        dir.join("rust/libcore.rmeta"),
        dir.join("rust/libcompiler_builtins.rmeta"),
        dir.join("rust/libkernel.rmeta"),
        dir.join("rust/libmacros.so"),
    ];
    if required.iter().any(|p| !p.exists()) {
        return false;
    }
    if paths.target.short_alias() && !dir.join("rust_sym_map.txt").exists() {
        return false;
    }
    true
}

fn fetch_prebuilt(paths: &BuildPaths, manifest: &Manifest) -> Result<(), String> {
    let repo = manifest
        .sdk
        .artifact_repo
        .clone()
        .unwrap_or_else(|| "Dere3046/RaanaSDK".to_string());
    let tag = manifest
        .sdk
        .artifact_tag
        .clone()
        .unwrap_or_else(|| format!("rust-support-{}", manifest.sdk.rust_support));

    let local_root = Path::new(&repo);
    let src_dir = if local_root.exists() {
        let candidates = [
            local_root.join(&tag).join(paths.target.name()),
            local_root.join(paths.target.name()),
            local_root.to_path_buf(),
        ];
        let dir = candidates
            .iter()
            .find(|d| d.join("rust_support.ko").exists())
            .cloned();

        if let Some(dir) = dir {
            dir
        } else {
            let asset = format!("rust-support-{}.tar.gz", paths.target.name());
            let tarball = [local_root.join(&tag).join(&asset), local_root.join(&asset)]
                .iter()
                .find(|p| p.exists())
                .cloned()
                .ok_or_else(|| format!("prebuilt not found under local path {}", repo))?;

            let tmp = std::env::temp_dir().join(format!("raana-prebuilt-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&tmp);
            let extract = tmp.join("extract");
            std::fs::create_dir_all(&extract).map_err(|e| e.to_string())?;
            run_cmd(
                "tar",
                &[
                    "-xzf",
                    tarball.to_str().ok_or("bad path")?,
                    "-C",
                    extract.to_str().ok_or("bad path")?,
                ],
                &paths.project_root,
            )?;
            extract
        }
    } else {
        let tmp = std::env::temp_dir().join(format!("raana-prebuilt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

        let asset = format!("rust-support-{}.tar.gz", paths.target.name());
        run_cmd(
            "gh",
            &[
                "release",
                "download",
                &tag,
                "--repo",
                &repo,
                "--pattern",
                &asset,
                "--dir",
                tmp.to_str().ok_or("bad path")?,
            ],
            &paths.project_root,
        )?;

        let tarball = tmp.join(&asset);
        if !tarball.exists() {
            return Err(format!("prebuilt asset {} not found", asset));
        }

        let extract = tmp.join("extract");
        std::fs::create_dir_all(&extract).map_err(|e| e.to_string())?;
        run_cmd(
            "tar",
            &[
                "-xzf",
                tarball.to_str().ok_or("bad path")?,
                "-C",
                extract.to_str().ok_or("bad path")?,
            ],
            &paths.project_root,
        )?;

        extract
    };

    let verify = manifest.sdk.verify.unwrap_or(true);
    if verify {
        let meta = src_dir.join("artifact.json");
        let content = std::fs::read_to_string(&meta)
            .map_err(|e| format!("artifact.json missing in prebuilt: {}", e))?;
        if !content.contains(&manifest.sdk.rust_support) {
            return Err(format!(
                "prebuilt rev mismatch: expected {}",
                manifest.sdk.rust_support
            ));
        }
    }

    let dir = paths.host_rust_support_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let copy_src = format!("{}/.", src_dir.display());
    run_cmd(
        "cp",
        &["-a", copy_src.as_str(), dir.to_str().ok_or("bad path")?],
        &paths.project_root,
    )
}

fn sync_makefile(manifest: &Manifest, project_root: &Path) -> Result<(), String> {
    let content = crate::scaffold::makefile_from_manifest(manifest, project_root)?;
    let path = project_root.join("Makefile");
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    if old != content {
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn build_hash(manifest: &Manifest, target: GkiTarget) -> String {
    let mut hasher = DefaultHasher::new();
    manifest.package.name.hash(&mut hasher);
    manifest.package.language.hash(&mut hasher);
    manifest.package.rust.hash(&mut hasher);
    manifest.package.wrapper.hash(&mut hasher);
    manifest.package.kunit.hash(&mut hasher);
    for src in &manifest.package.sources {
        src.hash(&mut hasher);
    }
    manifest.sdk.rust_support.hash(&mut hasher);
    target.name().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
