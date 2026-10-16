// A separate Cargo test executable: never included by the application library.
// Path-included production modules contain unit-test helpers that harness=false
// does not execute. Keep those warnings local to this test executable.
#![allow(dead_code, unused_imports)]
#[path = "../src/commands.rs"]
mod commands;
#[path = "../src/native_runtime.rs"]
mod native_runtime;
#[path = "../src/registration.rs"]
mod registration;
#[path = "../src/scans.rs"]
mod scans;
#[path = "../src/workspace_runtime.rs"]
mod workspace_runtime;
#[path = "../src/workspaces.rs"]
mod workspaces;
use registration::{application_context, application_handler};
#[path = "native_acceptance/mod.rs"]
mod native_acceptance;

fn main() {
    native_acceptance::main();
}
