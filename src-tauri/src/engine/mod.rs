//! Transfer orchestration layer — Bölüm 9, 14, 15.
//!
//! `TransferEngine` adapter + cancellation + events üçlüsünü birleştirir:
//! her `submit()` çağrısı bir background task spawn eder, adapter'ı çağırır,
//! progress'i `ProgressAggregator` üzerinden 250ms penceresinde batch'ler,
//! lifecycle event'lerini (Queued → Active → Completed/Failed/Cancelled)
//! `EventBus`'a yayınlar.
//!
//! Faz 2 kapsamı: tek seferlik transfer dispatch. Asıl multipart, retry,
//! rate limit, resume — Faz 3+'ta.

mod progress;
mod transfer_engine;
mod types;

pub use progress::ProgressAggregator;
pub use transfer_engine::TransferEngine;
pub use types::{TransferDirection, TransferHandle, TransferRequest};
