//! Engine'in dış API tipleri.
//!
//! Sade tutuluyor — Faz 2 yalnızca local-to-local + tek transfer akışı.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::cancellation::TransferCancellation;
use crate::errors::TransferError;
use crate::protocols::{LocalPath, ProtocolAdapter, RemotePath, TransferOptions, TransferResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferDirection {
    Upload,
    Download,
}

pub struct TransferRequest {
    pub id: Uuid,
    pub direction: TransferDirection,
    pub local: LocalPath,
    pub remote: RemotePath,
    pub adapter: Arc<dyn ProtocolAdapter>,
    pub options: TransferOptions,
}

impl TransferRequest {
    pub fn new(
        direction: TransferDirection,
        local: LocalPath,
        remote: RemotePath,
        adapter: Arc<dyn ProtocolAdapter>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            direction,
            local,
            remote,
            adapter,
            options: TransferOptions::default(),
        }
    }

    pub fn with_options(mut self, opts: TransferOptions) -> Self {
        self.options = opts;
        self
    }
}

/// `TransferEngine.submit()` çıktısı — cancel/wait için handle.
///
/// `JoinHandle`'ın iki yönü:
/// - `cancel()` `TransferCancellation` üzerinden token iptal eder
///   (cooperative — Bölüm 32.1).
/// - `wait()` join handle'ı bekler ve transfer sonucunu döner.
pub struct TransferHandle {
    pub id: Uuid,
    cancellation: TransferCancellation,
    join: JoinHandle<Result<TransferResult, TransferError>>,
}

impl TransferHandle {
    pub(crate) fn new(
        id: Uuid,
        cancellation: TransferCancellation,
        join: JoinHandle<Result<TransferResult, TransferError>>,
    ) -> Self {
        Self {
            id,
            cancellation,
            join,
        }
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    /// Sahip alt-sistem (scheduler) handle'ı `wait()` ile tüketmeden önce
    /// kendi cancel registry'sine kayıt etmek için clone alır. `TransferHandle`
    /// kendisi `Clone` değil (JoinHandle move-only), ama içerdeki cancellation
    /// token zaten `Clone` ve sahiplik paylaşılabilir.
    pub fn cancellation_handle(&self) -> crate::cancellation::TransferCancellation {
        self.cancellation.clone()
    }

    pub async fn wait(self) -> Result<TransferResult, TransferError> {
        match self.join.await {
            Ok(result) => result,
            Err(join_err) => Err(TransferError::Protocol {
                message: format!("transfer task panicked or was aborted: {join_err}"),
            }),
        }
    }
}
