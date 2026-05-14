//! Filesystem Edge-Case Matrisi — Bölüm 12.
//!
//! Network/protocol katmanı sağlam olsa bile filesystem semantik farkları
//! (Win vs Linux vs S3) sessizce data corruption üretir. Bu modül beş kritik
//! eksenin pure-Rust, sync, side-effect-free yardımcılarını barındırır:
//!
//! - [`symlink`]          — Bölüm 12.1, CVE-class symlink target hijack koruması
//! - [`normalized_path`]  — Bölüm 12.2, NFC/NFD farkı internal compare
//! - [`case_conflict`]    — Bölüm 12.3, Windows/macOS case-insensitive collision
//! - [`sanitize`]         — Bölüm 12.4, reserved name / invalid char rewrite
//! - [`path_length`]      — Bölüm 12.5, MAX_PATH ve UNC long-path classifier
//!
//! Üst katmanlar (LocalAdapter, sync engine, IPC) bu modülün enum'larını
//! tüketir; pure fonksiyonlar olduğu için async runtime, disk I/O veya global
//! state taşımaz — sadece input → karar.

pub mod case_conflict;
pub mod normalized_path;
pub mod path_length;
pub mod sanitize;
pub mod symlink;

pub use case_conflict::{CaseConflict, CaseConflictDetector};
pub use normalized_path::NormalizedPath;
pub use path_length::{classify_path_length, PathLengthClass};
pub use sanitize::{sanitize_for_target, Mutation, SanitizeResult, TargetOS};
pub use symlink::{
    follow_with_cycle_check, sanitize_symlink_target, SymlinkAction, SymlinkError, SymlinkPolicy,
};
