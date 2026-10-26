#![no_main]
// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
use libfuzzer_sys::fuzz_target;

// Note: valid-looking headers run Argon2 (up to the accepted maxima), so expect this target to be slow.
fuzz_target!(|data: &[u8]| {
    let _ = pwdx_core::import_backup(data, "fuzz passphrase");
});
