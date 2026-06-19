//! Desktop binary entry point.
//!
//! All application logic lives in the library crate (`lib.rs`) so the same code
//! can be linked by the mobile platform projects. This binary is a thin shim
//! that calls into it. On Windows, release builds suppress the extra console
//! window (debug builds keep it so logs/panics stay visible while developing).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gsb_connect_lib::run()
}
