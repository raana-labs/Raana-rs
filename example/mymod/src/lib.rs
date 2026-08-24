// SPDX-License-Identifier: GPL-2.0-only
/*
 * Copyright (C) 2026 dere3046
 */

#![no_std]

use kernel::prelude::*;

module! {
    type: MyMod,
    name: "mymod",
    author: "dere3046",
    description: "Rust LKM template",
    license: "GPL",
}

struct MyMod;

impl kernel::Module for MyMod {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("mymod loaded\n");
        Ok(Self)
    }
}

impl Drop for MyMod {
    fn drop(&mut self) {
        pr_info!("mymod unloaded\n");
    }
}
