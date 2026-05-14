//! `ProgressAggregator` — Bölüm 9.4.
//!
//! Adapter `ProgressTick`'leri yüksek frekansla emit eder (her chunk read/write).
//! UI bunları aynı hızda render edemez — 100 transfer × 8 chunk × 100 tick/sn =
//! 80K event/sn Vue'yu bitirir. Aggregator 250ms penceresinde **son** tick'i
//! tutar, interval geldiğinde tek bir `TransferProgress` event yayınlar.
//!
//! Ek olarak hız (`speed_bps`) ve ETA hesabı tick'ler arası delta'dan türetilir.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{interval, Instant};
use uuid::Uuid;

use crate::events::{EngineEvent, EventBus};
use crate::protocols::types::ProgressTick;

/// Batch window — Bölüm 9.4 sabit 250ms.
const FLUSH_INTERVAL: Duration = Duration::from_millis(250);

pub struct ProgressAggregator;

impl ProgressAggregator {
    /// `rx` `None` döndürene kadar tick topla, 250ms interval'de batch emit et.
    /// `rx` kapanınca son tick'i (varsa) flush et ve çık.
    pub async fn run(
        transfer_id: Uuid,
        mut rx: mpsc::Receiver<ProgressTick>,
        events: Arc<EventBus>,
    ) {
        let mut interval = interval(FLUSH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // İlk tick anında firelanmasın — bir aralık boş atılsın.
        interval.tick().await;

        let mut latest: Option<ProgressTick> = None;
        let mut last_emit_at = Instant::now();
        let mut last_emit_bytes: u64 = 0;

        loop {
            tokio::select! {
                biased;
                received = rx.recv() => {
                    match received {
                        Some(tick) => latest = Some(tick),
                        None => break,
                    }
                }
                _ = interval.tick() => {
                    if let Some(tick) = latest {
                        flush(tick, transfer_id, &events, &mut last_emit_at, &mut last_emit_bytes);
                    }
                }
            }
        }

        // Final flush — adapter bittiğinde son state'i kaçırma.
        if let Some(tick) = latest {
            flush(tick, transfer_id, &events, &mut last_emit_at, &mut last_emit_bytes);
        }
    }
}

fn flush(
    tick: ProgressTick,
    transfer_id: Uuid,
    events: &EventBus,
    last_emit_at: &mut Instant,
    last_emit_bytes: &mut u64,
) {
    let now = Instant::now();
    let elapsed = (now - *last_emit_at).as_secs_f64().max(0.001);
    let delta = tick.bytes_done.saturating_sub(*last_emit_bytes);
    let speed_bps = delta as f64 / elapsed;
    let remaining = tick.bytes_total.saturating_sub(tick.bytes_done);
    let eta_secs = if speed_bps > 0.0 {
        Some((remaining as f64 / speed_bps) as u64)
    } else {
        None
    };

    events.emit(EngineEvent::TransferProgress {
        transfer_id,
        bytes_done: tick.bytes_done,
        bytes_total: tick.bytes_total,
        speed_bps,
        eta_secs,
    });

    *last_emit_at = now;
    *last_emit_bytes = tick.bytes_done;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use std::sync::Arc;
    use tokio::time::sleep;

    #[tokio::test]
    async fn aggregates_into_window_then_flushes() {
        let bus = Arc::new(EventBus::new(64));
        let mut subscriber = bus.subscribe_ui();
        let (tx, rx) = mpsc::channel(64);
        let transfer_id = Uuid::new_v4();

        let aggregator = tokio::spawn(ProgressAggregator::run(
            transfer_id,
            rx,
            Arc::clone(&bus),
        ));

        // 50ms aralıklarla 6 tick gönder — toplam 300ms, 1+ window dolar.
        for i in 1u64..=6 {
            tx.send(ProgressTick {
                chunk_index: i as u32,
                bytes_done: i * 1024,
                bytes_total: 6 * 1024,
            })
            .await
            .unwrap();
            sleep(Duration::from_millis(50)).await;
        }
        drop(tx);
        aggregator.await.unwrap();

        // Best-effort: en az 1 event (final flush garantili) alınmalı.
        let mut events = Vec::new();
        while let Ok(ev) = subscriber.try_recv() {
            events.push(ev);
        }
        assert!(
            !events.is_empty(),
            "expected at least one TransferProgress event"
        );
        let last = events.last().unwrap();
        match &**last {
            EngineEvent::TransferProgress {
                bytes_done,
                bytes_total,
                ..
            } => {
                assert_eq!(*bytes_done, 6 * 1024);
                assert_eq!(*bytes_total, 6 * 1024);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn drops_silently_when_sender_closes_without_ticks() {
        let bus = Arc::new(EventBus::new(64));
        let (tx, rx) = mpsc::channel(64);
        drop(tx);
        ProgressAggregator::run(Uuid::new_v4(), rx, bus).await;
        // Hata almadan bitmesi yeterli — bu davranış no-tick edge case'i kapsar.
    }
}
