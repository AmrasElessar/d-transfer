//! Queue persistence katmanı — Bölüm 15.
//!
//! - [`schema`]        — SQLite DDL + migration runner (WAL mode pragma'ları)
//! - [`state_machine`] — `can_transition_to` validatörü (15.1)
//! - [`task`]          — `PersistedTransferTask` struct + row codec
//! - [`db_actor`]      — `DbActor` writer serileştirme (15.4)
//! - [`recovery`]      — Startup recovery (orphan Active → Queued)
//!
//! **Scope (Faz 3 — bu slice):** DB layer + actor. QueueScheduler entegrasyonu
//! (15.3) sonraki slice'ta. WAL checkpoint policy (15.7) ileride.

mod db_actor;
mod recovery;
mod schema;
mod state_machine;
mod task;

#[cfg(test)]
mod recovery_tests;

pub use db_actor::{spawn_db_actor, DbActorHandle, DbCommand, DbError};
pub use recovery::{cleanup_orphan_tmps, run_recovery, RecoveryReport};
pub use state_machine::can_transition_to;
pub use task::PersistedTransferTask;
