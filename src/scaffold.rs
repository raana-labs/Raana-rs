use std::path::Path;

use crate::config::ScaffoldConfig;
use crate::manifest::{Dependencies, Manifest};
use crate::Sdk;

pub fn create_project(name: &str, dir: &Path, cfg: &ScaffoldConfig) -> Result<(), String> {
    let crate_name = name.replace('-', "_");
    let module_type = to_camel_case(&crate_name);
    let sdk = Sdk::current();

    std::fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;

    std::fs::write(dir.join("lkm.toml"), lkm_toml(name, &sdk)).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("deps.lst"), deps_lst(&sdk)).map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join("Makefile"),
        makefile_with_deps(name, &Dependencies::default(), dir)?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(dir.join(".gitignore"), ".cache/\n.raana_cache/\nout/\n")
        .map_err(|e| e.to_string())?;
    std::fs::write(dir.join("src/lib.rs"), lib_rs(name, &module_type, cfg))
        .map_err(|e| e.to_string())?;
    std::fs::write(dir.join("src/wrapper.c"), wrapper_c(name, &crate_name, cfg))
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn create_c_project(name: &str, dir: &Path, cfg: &ScaffoldConfig) -> Result<(), String> {
    let sdk = Sdk::current();

    std::fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;

    std::fs::write(dir.join("lkm.toml"), c_lkm_toml(name, &sdk)).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("deps.lst"), c_deps_lst()).map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join("Makefile"),
        makefile_c_with_deps(
            name,
            &["src/main.c".to_string()],
            &Dependencies::default(),
            dir,
        )?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(dir.join(".gitignore"), ".cache/\n.raana_cache/\nout/\n")
        .map_err(|e| e.to_string())?;
    std::fs::write(dir.join("src/main.c"), c_main_c(name, cfg)).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn makefile_from_manifest(manifest: &Manifest, project_root: &Path) -> Result<String, String> {
    if manifest.package.language == "c" {
        makefile_c_with_deps(
            &manifest.package.name,
            &manifest.package.sources,
            &manifest.dependencies,
            project_root,
        )
    } else {
        makefile_with_deps(&manifest.package.name, &manifest.dependencies, project_root)
    }
}

fn lkm_toml(name: &str, sdk: &Sdk) -> String {
    format!(
        "[package]\nname = \"{}\"\nrust = \"src/lib.rs\"\nwrapper = \"src/wrapper.c\"\nkunit = false\n\n[sdk]\nkmsdk = \"{}\"\nrust-support = \"{}\"\n\n[build]\ntargets = [\"{}\"]\ncache = \".cache\"\n",
        name,
        sdk.kmsdk_rev,
        sdk.rust_support_rev,
        crate::config::DEFAULT_TARGET
    )
}

fn deps_lst(sdk: &Sdk) -> String {
    format!("# <name> <rev>\nrust_support {}\n", sdk.rust_support_rev)
}

fn makefile_with_deps(
    name: &str,
    deps: &Dependencies,
    project_root: &Path,
) -> Result<String, String> {
    let crate_name = name.replace('-', "_");

    let mut objs = vec![
        format!("{}_rust.o", crate_name),
        "src/wrapper.o".to_string(),
    ];
    let mut includes = Vec::new();

    for dep in deps.c.values() {
        for obj in &dep.objs {
            objs.push(format!("{}/{}", dep.path, obj));
        }
        for inc in &dep.includes {
            includes.push(format!("ccflags-y += -I$(src)/{}/{}", dep.path, inc));
        }
    }

    collect_kmsdk_deps(project_root, deps, &mut objs, &mut includes)?;

    let objs_line = objs.join(" ");
    let includes_lines = if includes.is_empty() {
        String::new()
    } else {
        includes.join("\n")
    };

    Ok(format!(
        "# SPDX-License-Identifier: GPL-2.0-only\n\nobj-m := {name}.o\n\nKDIR := $(KDIR)\nMDIR := $(realpath $(dir $(abspath $(lastword $(MAKEFILE_LIST)))))\nODIR := $(MDIR)/out/$(VER)\n\n{name}-y := {objs_line}\n\nccflags-y += -std=gnu11\nccflags-y += -Wno-declaration-after-statement\nccflags-y += -Wno-unused-variable\nccflags-y += -Wno-unused-function\nccflags-y += -Wno-strict-prototypes\n{includes_lines}\n\n$(info -- KDIR: $(KDIR))\n$(info -- MDIR: $(MDIR))\n$(info -- ODIR: $(ODIR))\n\nall:\n\tmake -C $(KDIR) M=$(ODIR) src=$(MDIR) modules\nclean:\n\tmake -C $(KDIR) M=$(ODIR) src=$(MDIR) clean\n\n$(obj)/%.o: $(src)/%.c $(recordmcount_source) FORCE\n\t$(call if_changed_rule,cc_o_c)\n\t$(call cmd,force_checksrc)\n",
        name = name,
        objs_line = objs_line,
        includes_lines = includes_lines
    ))
}

fn makefile_c_with_deps(
    name: &str,
    sources: &[String],
    deps: &Dependencies,
    project_root: &Path,
) -> Result<String, String> {
    let mut objs = sources
        .iter()
        .map(|s| s.trim_end_matches(".c").to_string() + ".o")
        .collect::<Vec<_>>();
    let mut includes = Vec::new();

    for dep in deps.c.values() {
        for obj in &dep.objs {
            objs.push(format!("{}/{}", dep.path, obj));
        }
        for inc in &dep.includes {
            includes.push(format!("ccflags-y += -I$(src)/{}/{}", dep.path, inc));
        }
    }

    collect_kmsdk_deps(project_root, deps, &mut objs, &mut includes)?;

    let objs_line = objs.join(" ");
    let includes_lines = if includes.is_empty() {
        String::new()
    } else {
        includes.join("\n")
    };

    Ok(format!(
        "# SPDX-License-Identifier: GPL-2.0-only\n\nobj-m := {name}.o\n\nKDIR := $(KDIR)\nMDIR := $(realpath $(dir $(abspath $(lastword $(MAKEFILE_LIST)))))\nODIR := $(MDIR)/out/$(VER)\n\n{name}-y := {objs_line}\n\nccflags-y += -std=gnu11\nccflags-y += -Wno-declaration-after-statement\nccflags-y += -Wno-unused-variable\nccflags-y += -Wno-unused-function\nccflags-y += -Wno-strict-prototypes\n{includes_lines}\n\n$(info -- KDIR: $(KDIR))\n$(info -- MDIR: $(MDIR))\n$(info -- ODIR: $(ODIR))\n\nall:\n\tmake -C $(KDIR) M=$(ODIR) src=$(MDIR) modules\nclean:\n\tmake -C $(KDIR) M=$(ODIR) src=$(MDIR) clean\n\n$(obj)/%.o: $(src)/%.c $(recordmcount_source) FORCE\n\t$(call if_changed_rule,cc_o_c)\n\t$(call cmd,force_checksrc)\n",
        name = name,
        objs_line = objs_line,
        includes_lines = includes_lines
    ))
}

fn collect_kmsdk_deps(
    project_root: &Path,
    deps: &Dependencies,
    objs: &mut Vec<String>,
    includes: &mut Vec<String>,
) -> Result<(), String> {
    let mut seen = Vec::new();
    for lib in &deps.kmsdk.libs {
        add_kmsdk_lib(lib, project_root, objs, includes, &mut seen)?;
    }
    Ok(())
}

fn add_kmsdk_lib(
    lib: &str,
    project_root: &Path,
    objs: &mut Vec<String>,
    includes: &mut Vec<String>,
    seen: &mut Vec<String>,
) -> Result<(), String> {
    if seen.iter().any(|s| s == lib) {
        return Ok(());
    }
    seen.push(lib.to_string());

    let meta = project_root.join("deps").join(lib).join("deps.mk");
    let content =
        std::fs::read_to_string(&meta).map_err(|e| format!("missing {} deps.mk: {}", lib, e))?;

    let mut lib_objs = Vec::new();
    let mut lib_includes = Vec::new();
    let mut lib_deps = Vec::new();

    for line in content.lines() {
        if let Some(v) = line.strip_prefix("DEPS_LIB_OBJS := ") {
            lib_objs = v.split_whitespace().map(|s| s.to_string()).collect();
        } else if let Some(v) = line.strip_prefix("DEPS_LIB_INCS := ") {
            lib_includes = v.split_whitespace().map(|s| s.to_string()).collect();
        } else if let Some(v) = line.strip_prefix("DEPS_LIB_DEPS := ") {
            lib_deps = v.split_whitespace().map(|s| s.to_string()).collect();
        }
    }

    for obj in lib_objs {
        objs.push(format!("deps/{}/{}", lib, obj));
    }
    for inc in lib_includes {
        includes.push(format!("ccflags-y += -I$(src)/deps/{}/{}", lib, inc));
    }
    for dep in lib_deps {
        add_kmsdk_lib(&dep, project_root, objs, includes, seen)?;
    }

    Ok(())
}

fn c_lkm_toml(name: &str, sdk: &Sdk) -> String {
    format!(
        "[package]\nname = \"{}\"\nlanguage = \"c\"\nsources = [\"src/main.c\"]\n\n[sdk]\nkmsdk = \"{}\"\nrust-support = \"{}\"\n\n[build]\ntargets = [\"{}\"]\ncache = \".cache\"\n",
        name,
        sdk.kmsdk_rev,
        sdk.rust_support_rev,
        crate::config::DEFAULT_TARGET
    )
}

fn c_deps_lst() -> String {
    "# <name> <rev>\n".to_string()
}

fn c_main_c(name: &str, cfg: &ScaffoldConfig) -> String {
    let header = cfg.render_c(name);
    format!(
        "{header}\n#include <linux/init.h>\n#include <linux/module.h>\n#include <linux/kernel.h>\n\nstatic int __init {name}_init(void)\n{{\n\tpr_info(\"{name} loaded\\n\");\n\treturn 0;\n}}\n\nstatic void __exit {name}_exit(void)\n{{\n\tpr_info(\"{name} unloaded\\n\");\n}}\n\nmodule_init({name}_init);\nmodule_exit({name}_exit);\nMODULE_LICENSE(\"GPL\");\nMODULE_AUTHOR(\"{author}\");\nMODULE_DESCRIPTION(\"C LKM\");\n",
        name = name,
        author = cfg.author
    )
}

fn lib_rs(name: &str, module_type: &str, cfg: &ScaffoldConfig) -> String {
    let header = cfg.render_rust(name);
    format!(
        "{header}\n#![no_std]\n\nuse kernel::prelude::*;\n\nmodule! {{\n    type: {module_type},\n    name: \"{name}\",\n    author: \"{author}\",\n    description: \"Rust LKM\",\n    license: \"GPL\",\n}}\n\nstruct {module_type};\n\nimpl kernel::Module for {module_type} {{\n    fn init(_module: &'static ThisModule) -> Result<Self> {{\n        pr_info!(\"{name} loaded\\n\");\n        Ok(Self)\n    }}\n}}\n\nimpl Drop for {module_type} {{\n    fn drop(&mut self) {{\n        pr_info!(\"{name} unloaded\\n\");\n    }}\n}}\n",
        name = name,
        module_type = module_type,
        author = cfg.author
    )
}

fn wrapper_c(name: &str, crate_name: &str, cfg: &ScaffoldConfig) -> String {
    let header = cfg.render_c(name);
    format!(
        "{header}\n#include <linux/init.h>\n#include <linux/module.h>\n\nextern int rust_{crate_name}_init_module(void);\nextern void rust_{crate_name}_cleanup_module(void);\n\nint __init init_module(void)\n{{\n\treturn rust_{crate_name}_init_module();\n}}\n\nvoid __exit cleanup_module(void)\n{{\n\trust_{crate_name}_cleanup_module();\n}}\n\n/*\n * old CFI needs __cfi_jt_* jump table entries\n * module_init/module_exit macros emit them\n * manual wrapper must add them\n */\n#ifdef __CFI_ADDRESSABLE\n__CFI_ADDRESSABLE(init_module, __initdata);\n__CFI_ADDRESSABLE(cleanup_module, __exitdata);\n#endif\n",
        crate_name = crate_name
    )
}

fn to_camel_case(name: &str) -> String {
    name.split('_')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}
