use std::path::{Path, PathBuf};
use std::process::Command;

use raana::manifest::Manifest;
use raana::scaffold::{create_c_project, create_project};
use raana::{
    build_from_manifest, compile_module, fetch_project, link_module, BuildPaths, GkiTarget,
};

fn print_targets() {
    println!(
        "{:<18} {:<8} {:<8} {:<8} {:<8}",
        "target", "alias", "fmt", "helper", "modpost"
    );
    for t in GkiTarget::all() {
        println!(
            "{:<18} {:<8} {:<8} {:<8} {:<8}",
            t.name(),
            t.short_alias(),
            t.skip_rust_fmt(),
            t.skip_rust_helpers(),
            t.modpost_fix()
        );
    }
}

fn print_usage() {
    println!("usage: raana <targets|fetch|install|sdk|doctor|new|build>");
    println!("  sdk <args...>");
    println!("  doctor");
    println!("  new [--c] [--author NAME] [--email EMAIL] [--year YEAR] [--license SPDX]");
    println!("      [--header-file PATH] [--header-c-file PATH] [--header-rust-file PATH] <name>");
    println!("  build [--manifest PATH] [--target NAME] [--project DIR] [--cache DIR]");
    println!("        [--module NAME] [--rust-src REL] [--kunit] [--docker|--host]");
}

fn find_arg(args: &[String], name: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == name && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        i += 1;
    }
    None
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn find_positional(args: &[String], value_flags: &[&str]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with("--") {
            if value_flags.contains(&arg.as_str()) {
                i += 2;
            } else {
                i += 1;
            }
        } else {
            return Some(arg.clone());
        }
    }
    None
}

fn build_with_flags(args: &[String]) -> Result<(), String> {
    let manifest_path = find_arg(args, "--manifest");
    if let Some(path) = manifest_path {
        let path = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
        let manifest = Manifest::load(&path)?;
        let project = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let target_name = find_arg(args, "--target").unwrap_or_else(|| {
            manifest
                .targets()
                .first()
                .map(|t| t.name().to_string())
                .unwrap_or_else(|| raana::config::DEFAULT_TARGET.to_string())
        });
        let target = GkiTarget::from_name(&target_name)
            .ok_or_else(|| format!("unknown target {}", target_name))?;
        let ko = build_from_manifest(&manifest, &project, target)?;
        println!("{}", ko.display());
        return Ok(());
    }

    let target_name =
        find_arg(args, "--target").unwrap_or_else(|| raana::config::DEFAULT_TARGET.to_string());
    let target = GkiTarget::from_name(&target_name)
        .ok_or_else(|| format!("unknown target {}", target_name))?;
    let project = find_arg(args, "--project")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let cache = find_arg(args, "--cache")
        .map(PathBuf::from)
        .unwrap_or_else(|| project.join(raana::config::DEFAULT_CACHE_DIR));
    let module = find_arg(args, "--module").unwrap_or_else(|| "mymod".to_string());
    let rust_src = find_arg(args, "--rust-src").unwrap_or_else(|| "src/lib.rs".to_string());
    let kunit = has_flag(args, "--kunit");

    let runtime = raana::config::RuntimeConfig::from_env();
    let use_container_paths = if has_flag(args, "--docker") {
        true
    } else if has_flag(args, "--host") {
        false
    } else {
        !raana::has_ddk_host(target, &runtime)
    };
    if !use_container_paths && !raana::has_ddk_host(target, &runtime) {
        return Err("host mode requires /opt/ddk with kdir and rust toolchain".to_string());
    }
    let project = project.canonicalize().map_err(|e| e.to_string())?;
    let paths = BuildPaths::new(
        project,
        cache,
        target,
        runtime.rust_support_rev.clone(),
        None,
        runtime,
        use_container_paths,
    );

    compile_module(&paths, &module, &rust_src, kunit)?;
    let ko = link_module(&paths, &module)?;
    println!("{}", ko.display());
    Ok(())
}

fn run_fetch(args: &[String]) -> Result<(), String> {
    let manifest_path = find_arg(args, "--manifest")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("lkm.toml"));
    let manifest_path = std::fs::canonicalize(&manifest_path)
        .map_err(|e| format!("cannot open manifest: {}", e))?;
    let manifest = Manifest::load(&manifest_path)?;
    let project = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    fetch_project(&project, &manifest)
}

fn run_sdk(args: &[String]) -> Result<(), String> {
    let project = if let Some(path) = find_arg(args, "--manifest") {
        let path = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| e.to_string())?
    };

    let sdk_script = project.join(".sdk/scripts/sdk");
    if !sdk_script.exists() {
        return Err("SDK not installed, run `raana fetch` first".to_string());
    }

    let mut sdk_args = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--manifest" {
            i += 2;
        } else {
            sdk_args.push(args[i].as_str());
            i += 1;
        }
    }

    let status = Command::new(&sdk_script)
        .args(&sdk_args)
        .current_dir(&project)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("sdk failed with {}", status))
    }
}

fn run_doctor() -> Result<(), String> {
    let project = std::env::current_dir().map_err(|e| e.to_string())?;
    let sdk_script = project.join(".sdk/scripts/sdk");
    let deps_lst = project.join("deps.lst");
    let rust_support = project.join("deps/rust_support");

    println!("project {}", project.display());
    if sdk_script.exists() {
        println!("sdk installed");
    } else {
        println!("sdk missing");
    }
    if deps_lst.exists() {
        println!("deps.lst present");
    } else {
        println!("deps.lst missing");
    }
    if rust_support.exists() {
        println!("rust_support present");
    } else {
        println!("rust_support missing");
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");

    match cmd {
        "targets" => print_targets(),
        "new" => {
            let c = has_flag(&args[1..], "--c");
            let value_flags = [
                "--author",
                "--email",
                "--year",
                "--license",
                "--header-file",
                "--header-c-file",
                "--header-rust-file",
            ];
            let name = find_positional(&args[1..], &value_flags).unwrap_or_default();
            if name.is_empty() {
                print_usage();
                std::process::exit(1);
            }

            let mut cfg = raana::config::ScaffoldConfig::from_env();
            cfg.apply_user_config();

            if let Some(author) = find_arg(&args[1..], "--author") {
                cfg.author = author;
            }
            if let Some(email) = find_arg(&args[1..], "--email") {
                cfg.email = email;
            }
            if let Some(year) = find_arg(&args[1..], "--year") {
                cfg.year = year;
            }
            if let Some(spdx) = find_arg(&args[1..], "--license") {
                cfg.license_spdx = spdx;
            }
            if let Some(path) = find_arg(&args[1..], "--header-file") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    cfg.header_c = content.clone();
                    cfg.header_rust = content;
                }
            }
            if let Some(path) = find_arg(&args[1..], "--header-c-file") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    cfg.header_c = content;
                }
            }
            if let Some(path) = find_arg(&args[1..], "--header-rust-file") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    cfg.header_rust = content;
                }
            }

            let dir = PathBuf::from(&name);
            let result = if c {
                create_c_project(&name, &dir, &cfg)
            } else {
                create_project(&name, &dir, &cfg)
            };
            if let Err(e) = result {
                eprintln!("new failed: {}", e);
                std::process::exit(1);
            }
            println!("created {}", dir.display());
        }
        "fetch" | "install" => {
            if let Err(e) = run_fetch(&args[1..]) {
                eprintln!("fetch failed: {}", e);
                std::process::exit(1);
            }
        }
        "sdk" => {
            if let Err(e) = run_sdk(&args[1..]) {
                eprintln!("sdk failed: {}", e);
                std::process::exit(1);
            }
        }
        "doctor" => {
            if let Err(e) = run_doctor() {
                eprintln!("doctor failed: {}", e);
                std::process::exit(1);
            }
        }
        "build" => {
            if let Err(e) = build_with_flags(&args[1..]) {
                eprintln!("build failed: {}", e);
                std::process::exit(1);
            }
        }
        _ => print_usage(),
    }
}
