use std::path::{Path, PathBuf};

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
    println!("usage: raana <targets|fetch|new|build>");
    println!("  new [--c] <name>");
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
        .unwrap_or_else(|| {
            PathBuf::from("/home/Dere3046/code/devLKM/rust_support/RANASL/test/rust_hello")
        });
    let cache = find_arg(args, "--cache")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/Dere3046/code/devLKM/rust_support/raana/.cache"));
    let module = find_arg(args, "--module").unwrap_or_else(|| "hello_rust".to_string());
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");

    match cmd {
        "targets" => print_targets(),
        "new" => {
            let c = has_flag(&args[1..], "--c");
            let name = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with('-'))
                .map(String::as_str)
                .unwrap_or("");
            if name.is_empty() {
                print_usage();
                std::process::exit(1);
            }
            let dir = PathBuf::from(name);
            let result = if c {
                create_c_project(name, &dir)
            } else {
                create_project(name, &dir)
            };
            if let Err(e) = result {
                eprintln!("new failed: {}", e);
                std::process::exit(1);
            }
            println!("created {}", dir.display());
        }
        "fetch" => {
            let manifest_path = find_arg(&args[1..], "--manifest")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("lkm.toml"));
            let manifest_path = match std::fs::canonicalize(&manifest_path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("fetch: cannot open manifest: {}", e);
                    std::process::exit(1);
                }
            };
            let manifest = match Manifest::load(&manifest_path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("fetch: {}", e);
                    std::process::exit(1);
                }
            };
            let project = manifest_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();
            if let Err(e) = fetch_project(&project, &manifest) {
                eprintln!("fetch failed: {}", e);
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
