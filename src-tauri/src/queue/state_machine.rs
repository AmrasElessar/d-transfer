//! Transfer state geçiş validatörü — Bölüm 15.1 + 15.3 türevleri.
//!
//! Geçersiz transition'lar race condition + double retry koruması sağlar.
//! Terminal state'lerden (Completed/Cancelled/Skipped) çıkış yasaktır;
//! Failed → Queued **tek istisna** (manuel/otomatik retry akışı).

use crate::events::TransferState;

/// `from` state'inden `to` state'ine geçiş izinli mi?
///
/// **Spec referansı (Bölüm 15.1):** Ana matris orada `Queued/Active/Paused/
/// Completed/Failed/Cancelled` üzerinden tanımlı. Bizim enum'ımız ek olarak
/// `Verifying`, `Finalizing`, `Skipped` taşıdığı için commit pipeline'ına
/// uygun geçişler de eklendi:
///
/// - `Active → Verifying → Finalizing → Completed` doğal commit yolu.
/// - `Verifying/Finalizing → Failed` her commit fazından başarısızlık.
/// - `Queued → Skipped` filter/skip rule'larından sonra terminal'e atlama.
pub fn can_transition_to(from: TransferState, to: TransferState) -> bool {
    use TransferState::*;
    match (from, to) {
        // Queued başlangıç noktası
        (Queued, Active) => true,
        (Queued, Cancelled) => true,
        (Queued, Skipped) => true,
        // Queued → Failed: dispatch öncesi hata (adapter factory unavailable,
        // profile not found, vb). Task hiç Active olmadan terminal'e düşebilir.
        (Queued, Failed) => true,

        // Active çalışma fazı
        (Active, Verifying) => true,
        (Active, Paused) => true,
        (Active, Failed) => true,
        (Active, Cancelled) => true,
        // Active → Completed: küçük dosyada Verifying/Finalizing atlanabilir.
        (Active, Completed) => true,

        // Commit pipeline
        (Verifying, Finalizing) => true,
        (Verifying, Failed) => true,
        (Verifying, Cancelled) => true,
        (Finalizing, Completed) => true,
        (Finalizing, Failed) => true,

        // Pause/resume
        (Paused, Queued) => true,
        (Paused, Active) => true,
        (Paused, Cancelled) => true,

        // Retry: Failed → Queued (Bölüm 15.1 explicit izin).
        (Failed, Queued) => true,

        // Terminal: Completed, Cancelled, Skipped çıkış yok.
        // Self-transition da yok (idempotency caller sorumluluğunda).
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TransferState::*;

    #[test]
    fn queued_to_active_is_valid() {
        assert!(can_transition_to(Queued, Active));
    }

    #[test]
    fn active_to_completed_is_valid() {
        assert!(can_transition_to(Active, Completed));
    }

    #[test]
    fn completed_to_active_is_invalid() {
        // Terminal state'ten geri dönüş yok
        assert!(!can_transition_to(Completed, Active));
    }

    #[test]
    fn cancelled_to_queued_is_invalid() {
        // Cancelled terminal — kullanıcı yeni task açmalı
        assert!(!can_transition_to(Cancelled, Queued));
    }

    #[test]
    fn failed_to_queued_is_valid_for_retry() {
        // Tek terminal-out istisna: retry akışı
        assert!(can_transition_to(Failed, Queued));
    }

    #[test]
    fn verifying_pipeline_is_valid() {
        assert!(can_transition_to(Active, Verifying));
        assert!(can_transition_to(Verifying, Finalizing));
        assert!(can_transition_to(Finalizing, Completed));
    }

    #[test]
    fn self_transition_is_invalid() {
        // İdempotency caller'da; aynı state'e geçiş validate'ten geçmez
        assert!(!can_transition_to(Active, Active));
        assert!(!can_transition_to(Queued, Queued));
    }
}
