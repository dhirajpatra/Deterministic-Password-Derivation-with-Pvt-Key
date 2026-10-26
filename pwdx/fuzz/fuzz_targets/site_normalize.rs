#![no_main]
// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
use libfuzzer_sys::fuzz_target;

fuzz_target!(|s: &str| {
    if let Ok(a) = pwdx_core::normalize_site(s) {
        // idempotence is a property of the spec
        assert_eq!(pwdx_core::normalize_site(&a).as_deref(), Ok(a.as_str()));
    }
});
