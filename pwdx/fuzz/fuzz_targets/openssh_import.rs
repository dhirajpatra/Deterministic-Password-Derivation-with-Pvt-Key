#![no_main]
// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = pwdx_core::import_openssh(data, None);
    let _ = pwdx_core::import_openssh(data, Some(b"fuzz"));
});
