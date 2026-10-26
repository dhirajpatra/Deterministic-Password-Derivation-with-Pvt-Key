// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
//! FFI crate for the Flutter app.
//!
//! `api` is the only module the code generator reads. It contains no cryptography: every function forwards to
//! `pwdx-core`. The generated glue (`frb_generated.rs`, created by `flutter_rust_bridge_codegen generate`)
//! uses `unsafe`, so unlike `pwdx-core` this crate does not forbid unsafe code at the crate root; the `api`
//! module itself does.

pub mod api;
