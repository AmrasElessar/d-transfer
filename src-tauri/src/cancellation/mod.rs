//! CancellationToken hiyerarşisi — Bölüm 32.
//!
//! Tek bir `CancellationToken` tree iptali yönetir:
//!
//! ```text
//! AppCancellation (root)
//!   └── ProfileCancellation        (bağlantı kopar → tüm transferler iptal)
//!         └── TransferCancellation (tek transfer iptal)
//!               └── ChunkCancellation (tek chunk iptal)
//! ```
//!
//! Üst düzey iptal alt seviyelerin tamamını otomatik etkiler (child_token zinciri).
//! Per-stage cancel davranışı Bölüm 32.1'de yazılı — bu modül yalnızca token
//! propagation katmanını sağlar; "immediate/cooperative/deferred/non-cancellable"
//! ayrımı caller'lara aittir.

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppCancellation {
    token: CancellationToken,
}

impl AppCancellation {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub fn child_profile(&self, profile_id: Uuid) -> ProfileCancellation {
        ProfileCancellation {
            profile_id,
            token: self.token.child_token(),
        }
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

impl Default for AppCancellation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ProfileCancellation {
    pub profile_id: Uuid,
    token: CancellationToken,
}

impl ProfileCancellation {
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub fn child_transfer(&self, transfer_id: Uuid) -> TransferCancellation {
        TransferCancellation {
            transfer_id,
            token: self.token.child_token(),
        }
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

#[derive(Debug, Clone)]
pub struct TransferCancellation {
    pub transfer_id: Uuid,
    token: CancellationToken,
}

impl TransferCancellation {
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub fn child_chunk(&self, chunk_index: u32) -> ChunkCancellation {
        ChunkCancellation {
            chunk_index,
            token: self.token.child_token(),
        }
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

#[derive(Debug, Clone)]
pub struct ChunkCancellation {
    pub chunk_index: u32,
    token: CancellationToken,
}

impl ChunkCancellation {
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_cancels_child() {
        let app = AppCancellation::new();
        let profile = app.child_profile(Uuid::new_v4());
        let transfer = profile.child_transfer(Uuid::new_v4());
        let chunk = transfer.child_chunk(0);

        assert!(!chunk.is_cancelled());
        app.cancel();
        assert!(chunk.is_cancelled(), "app cancel must propagate to chunk");
        assert!(transfer.is_cancelled());
        assert!(profile.is_cancelled());
    }

    #[test]
    fn sibling_isolated() {
        let app = AppCancellation::new();
        let profile_a = app.child_profile(Uuid::new_v4());
        let profile_b = app.child_profile(Uuid::new_v4());

        profile_a.cancel();
        assert!(profile_a.is_cancelled());
        assert!(!profile_b.is_cancelled(), "sibling must not cancel");
        assert!(!app.is_cancelled(), "child cancel must not bubble up");
    }

    #[test]
    fn transfer_cancel_propagates_to_chunks() {
        let app = AppCancellation::new();
        let profile = app.child_profile(Uuid::new_v4());
        let transfer = profile.child_transfer(Uuid::new_v4());
        let chunk0 = transfer.child_chunk(0);
        let chunk1 = transfer.child_chunk(1);

        transfer.cancel();
        assert!(chunk0.is_cancelled());
        assert!(chunk1.is_cancelled());
        assert!(!profile.is_cancelled());
    }
}
