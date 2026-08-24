// SPDX-License-Identifier: GPL-2.0-only
/*
 * Copyright (C) 2026 dere3046
 */

#include <linux/init.h>
#include <linux/module.h>

extern int rust_mymod_init_module(void);
extern void rust_mymod_cleanup_module(void);

int __init init_module(void)
{
	return rust_mymod_init_module();
}

void __exit cleanup_module(void)
{
	rust_mymod_cleanup_module();
}

/*
 * old CFI needs __cfi_jt_* jump table entries
 * module_init/module_exit macros emit them
 * manual wrapper must add them
 */
__CFI_ADDRESSABLE(init_module, __initdata);
__CFI_ADDRESSABLE(cleanup_module, __exitdata);
