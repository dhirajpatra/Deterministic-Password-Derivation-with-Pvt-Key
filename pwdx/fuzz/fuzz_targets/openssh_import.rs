#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = pwdx_core::import_openssh(data, None);
    let _ = pwdx_core::import_openssh(data, Some(b"fuzz"));
});
