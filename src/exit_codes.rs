//! Standard process exit codes for the NODS CLI.
//!
//! These let scripts and CI pipelines distinguish failure types. They mirror
//! the surface of the Tillio CLI so tooling can treat the Huntington Applied CLIs uniformly.
#![allow(dead_code)] // SIGINT is part of the documented contract even if unused here.

pub const OK: i32 = 0;
pub const GENERAL: i32 = 1;
pub const AUTH: i32 = 2;
pub const NOT_FOUND: i32 = 3;
pub const VALIDATION: i32 = 4;
pub const NETWORK: i32 = 5;
pub const SERVER: i32 = 6;
pub const SIGINT: i32 = 130;
