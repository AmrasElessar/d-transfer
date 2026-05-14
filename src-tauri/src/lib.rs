//! DTransfer — deterministic, crash-resilient transfer infrastructure client.
//!
//! Bu crate Tauri 2 host process'i, Rust core engine'i ve IPC katmanını barındırır.
//! Module ayrımı teknik dokümandaki bölüm numaralarına göre yapılmıştır:
//!
//! - [`errors`]        — Bölüm 10  Structured Error Taxonomy
//! - [`events`]        — Bölüm 33  Unified EngineEvent Bus
//! - [`cancellation`]  — Bölüm 32  CancellationToken Standardizasyonu
//! - [`protocols`]     — Bölüm 9.1, 11  ProtocolAdapter trait + types
//! - [`fs_edge`]       — Bölüm 12  Filesystem Edge-Case Matrisi
//! - [`engine`]        — Bölüm 9, 14  TransferEngine + ProgressAggregator
//! - [`queue`]         — Bölüm 15.1-15.4 + 28  Queue persistence, DbActor, orphan tmp cleanup
//! - [`profiles`]      — Bölüm 25  ConnectionProfile + CredentialVault + AdapterFactory
//! - [`scheduler`]     — Bölüm 15.3  QueueScheduler dispatch loop
//! - [`settings`]      — Disk-persistent uygulama tercihleri (theme/locale hariç)
//! - [`diagnostics`]   — Bölüm 30  RuntimeMetrics + RuntimeLimits + LimitProfile + log retention
//! - [`rate_limit`]    — Bölüm 16  RateLimiter + adaptive backoff
//! - [`audit`]         — Bölüm 17  Audit Trail + MaskingEngine + Redacted<T>
//! - [`network`]       — Bölüm 37, 38  SshKeepalive + ProxyConfig + bypass
//! - [`commands`]      — Tauri IPC command layer

pub mod audit;
pub mod cancellation;
pub mod commands;
pub mod diagnostics;
pub mod engine;
pub mod errors;
pub mod events;
pub mod fs_edge;
pub mod network;
pub mod profiles;
pub mod protocols;
pub mod queue;
pub mod rate_limit;
pub mod scheduler;
pub mod settings;

use std::sync::Arc;

use tauri::{Emitter, Manager, RunEvent};
use tracing::{error, info, warn};

pub use errors::{ErrorCategory, TransferError};

/// Application-wide handle shared across Tauri commands.
///
/// `setup()` içinde tam olarak inşa edilir; field'lar `Arc` üzerinden paylaşılır
/// (scheduler worker'ı da aynı handle'lara ortak olur).
pub struct AppState {
    pub events: Arc<events::EventBus>,
    pub root_cancel: cancellation::AppCancellation,
    pub queue: Arc<queue::DbActorHandle>,
    pub factory: Arc<profiles::LocalAdapterFactory>,
    pub engine: Arc<engine::TransferEngine>,
    pub scheduler: scheduler::QueueScheduler,
    pub settings: Arc<settings::SettingsStore>,
    /// OS keystore aracılığı (Bölüm 25.1). `Arc` çünkü tüm command'lar
    /// paylaşılan handle üzerinden okuma/yazma yapar; vault stateless ama
    /// tip kararlılığı için Arc'lıyoruz.
    pub credentials: Arc<profiles::CredentialVault>,
    /// UI-facing kalıcı adapter havuzu (Bölüm 25, Faz 4). Profil seçildiğinde
    /// remote pane'in `list_remote_dir` çağrıları bu manager'a düşer; her
    /// listing'de yeniden SSH handshake yapılmasın diye Arc cache'liyoruz.
    pub connections: Arc<profiles::ConnectionManager>,
    /// Startup'ta detect edilen profile + uygulanan kaynak sınırları
    /// (Bölüm 30). Scheduler concurrency, S3 pool boyutları, tokio blocking
    /// thread sayısı bu değerlerden türer.
    pub limit_profile: diagnostics::LimitProfile,
    pub limits: diagnostics::RuntimeLimits,
    /// Adapter'ların provider yanıt header'larından beslediği rate-limit
    /// cache'i (Bölüm 16). Key = profile_id.
    pub rate_limiter: Arc<rate_limit::RateLimiter>,
    /// Opt-in audit trail (Bölüm 17). Engine startup'ta spawn edilir ama
    /// `settings.audit_enabled` true olmadan kimse `emit()` çağırmaz —
    /// producer kapısı UI tarafında settings gate'iyle açılır.
    pub audit: Arc<audit::AuditEngine>,
}

/// Entry point invoked from `main.rs`. Builds the tokio runtime per Bölüm 9.5,
/// installs tracing per Bölüm 35, then hands off to Tauri.
pub fn run() {
    init_tracing();

    // Bölüm 30.3-30.4: adaptive limit profile startup'ta probe edilir;
    // tokio blocking pool boyutu ondan türer (Workstation'da Desktop default'u
    // yetersiz olabilir).
    let limit_profile = diagnostics::LimitProfile::detect();
    let limits = limit_profile.to_limits();
    info!(?limit_profile, blocking_threads = limits.max_blocking_threads, "limit profile detected");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get().max(2))
        .max_blocking_threads(limits.max_blocking_threads)
        .thread_name("dtransfer-worker")
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    // Tauri picks up the ambient runtime via tauri::async_runtime::set.
    tauri::async_runtime::set(runtime.handle().clone());

    // Enter the tokio runtime context on the main thread so that the setup()
    // callback can call `tokio::task::spawn_blocking` (used by DbActor) without
    // panicking. EnterGuard lives until the end of `run()`.
    let _runtime_guard = runtime.enter();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            // Mevcut pencereyi öne getir + komut satırı argümanlarını forward et (Bölüm 6).
            info!(?args, ?cwd, "single-instance: existing instance focused");
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
                let _ = win.unminimize();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::engine_status,
            commands::probe_local_adapter,
            commands::start_local_transfer,
            commands::get_settings,
            commands::update_settings,
            commands::list_local_dir,
            commands::list_local_drives,
            commands::home_dir,
            commands::list_profiles,
            commands::create_profile,
            commands::update_profile,
            commands::delete_profile,
            commands::test_connection,
            commands::connect_profile,
            commands::disconnect_profile,
            commands::list_remote_dir,
            commands::list_transfers,
            commands::enqueue_upload,
            commands::enqueue_download,
            commands::cancel_transfer,
        ])
        .setup(move |app| {
            initialize_app_state(app, limit_profile, limits)?;
            spawn_event_bridge(app);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            RunEvent::ExitRequested { .. } => {
                info!("exit requested — signalling root cancellation");
                if let Some(state) = app_handle.try_state::<AppState>() {
                    state.root_cancel.cancel();
                }
            }
            RunEvent::Exit => {
                info!("dtransfer host process exiting");
            }
            _ => {}
        });
}

fn initialize_app_state(
    app: &tauri::App,
    limit_profile: diagnostics::LimitProfile,
    limits: diagnostics::RuntimeLimits,
) -> Result<(), Box<dyn std::error::Error>> {
    // queue.db konumu: per-user app data dir. Bölüm 6 — per-user kurulum
    // varsayılanıyla tutarlı, UAC/sudo gerektirmez.
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app_data_dir failed: {e}"))?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("create app_data_dir failed: {e}"))?;
    let db_path = data_dir.join("queue.db");
    info!(?db_path, "opening queue.db");

    let queue_handle = queue::spawn_db_actor(&db_path)
        .map_err(|e| format!("spawn DbActor failed: {e}"))?;
    let queue = Arc::new(queue_handle);

    // Settings store — settings.json yoksa default değerlerle oluşturulur.
    let settings_store = settings::SettingsStore::load_or_init(&data_dir)
        .map_err(|e| format!("settings load_or_init failed: {e}"))?;
    let settings_store = Arc::new(settings_store);
    info!("settings loaded from disk");

    let root_cancel = cancellation::AppCancellation::new();

    // Audit engine (Bölüm 17.2). audit.db queue.db'den ayrı — KVKK silme hakkı
    // talebinde audit verisi tek başına silinir, transfer kuyruğunu etkilemez.
    // Worker root_cancel ile drain edilir; producer'lar settings gate'inden
    // sonra emit edecek.
    let audit_path = data_dir.join("audit.db");
    let audit_engine = audit::AuditEngine::spawn(audit_path.clone(), root_cancel.token().clone())
        .map_err(|e| format!("audit engine spawn failed: {e}"))?;
    let audit = Arc::new(audit_engine);
    info!(?audit_path, "audit engine ready");

    // Startup recovery: DB tarafı (`Active → Queued`) + filesystem tarafı
    // (`*.dtransfer_tmp` orphan cleanup, mtime > 24h). İkisi paralel
    // koşmaya değecek kadar büyük değil; tek async task'te ardışık.
    let queue_for_recovery = Arc::clone(&queue);
    let events_pre = Arc::new(events::EventBus::new(1024));
    let events_for_recovery = Arc::clone(&events_pre);
    let scan_root = settings_store
        .snapshot()
        .default_download_dir
        .clone()
        .map(|p| vec![data_dir.clone(), p])
        .unwrap_or_else(|| vec![data_dir.clone()]);
    tauri::async_runtime::spawn(async move {
        match queue_for_recovery.recover().await {
            Ok(mut report) => {
                let removed = queue::cleanup_orphan_tmps(
                    &scan_root,
                    std::time::Duration::from_secs(24 * 3600),
                );
                report.orphan_tmp_files = removed;
                info!(
                    resurrected = report.resurrected_count,
                    abandoned = report.abandoned_count,
                    orphan_tmp = report.orphan_tmp_files,
                    "startup recovery complete"
                );
                events_for_recovery.emit(events::EngineEvent::QueueRecovered {
                    restored_count: report.resurrected_count,
                });
            }
            Err(e) => {
                error!(?e, "startup recovery failed");
            }
        }
    });

    let engine = Arc::new(engine::TransferEngine::new(
        Arc::clone(&events_pre),
        root_cancel.clone(),
    ));
    let factory = Arc::new(profiles::LocalAdapterFactory::new());

    // Credential vault — stateless wrapper, her komutta yeniden Entry kurar.
    // OS keystore arızalı bile olsa start-up'ı bozmayız; hata profile CRUD
    // anında upstream'e gider (Bölüm 25.1.3 Linux fallback notu).
    let credentials = Arc::new(profiles::CredentialVault::new());

    // Connection pool — vault'u paylaşır, SFTP password fetch için aynı keystore.
    let connections = Arc::new(profiles::ConnectionManager::new(Arc::clone(&credentials)));

    // Unified factory: scheduler dispatch path'i hem debug LocalAdapterFactory
    // (start_local_transfer) hem DB-backed profile'lar (enqueue_upload/download)
    // ile uyumlu çalışsın diye iki kaynağı birleştirir.
    let unified_factory = Arc::new(profiles::UnifiedAdapterFactory::new(
        Arc::clone(&factory),
        Arc::clone(&queue),
        Arc::clone(&connections),
    ));
    let factory_dyn: Arc<dyn profiles::AdapterFactory> =
        Arc::clone(&unified_factory) as Arc<dyn profiles::AdapterFactory>;

    let (scheduler, worker) = scheduler::new_scheduler(
        Arc::clone(&queue),
        Arc::clone(&engine),
        factory_dyn,
        Arc::clone(&settings_store),
        root_cancel.token().clone(),
    );

    // Scheduler worker — root_cancel iptal olunca temiz kapanır.
    tauri::async_runtime::spawn(worker.run());

    // Rate limiter (Bölüm 16) — adapter'lar her API yanıtında
    // `update_from_headers(profile_id, ...)` çağrısı yapacak.
    let rate_limiter = Arc::new(rate_limit::RateLimiter::default());

    let app_state = AppState {
        events: events_pre,
        root_cancel,
        queue,
        factory,
        engine,
        scheduler,
        settings: settings_store,
        credentials,
        connections,
        limit_profile,
        limits,
        rate_limiter,
        audit,
    };
    app.manage(app_state);
    Ok(())
}

fn spawn_event_bridge(app: &tauri::App) {
    // Bridge EventBus broadcast → Tauri webview emit (Bölüm 33).
    let handle = app.handle().clone();
    let events = app.state::<AppState>().events.clone();
    tauri::async_runtime::spawn(async move {
        let mut rx = events.subscribe_ui();
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Err(e) = handle.emit("engine-event", &*event) {
                        warn!(?e, "failed to forward engine-event to UI");
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "ui subscriber lagged — events dropped");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    // Bölüm 17.3.1 PII koruması: hassas modüller diagnostics bundle dahil
    // her ortamda **off**. Production'da `RUST_LOG=debug` set edilse bile
    // bu direktifler override edilemez (RUST_LOG'tan sonra `.add_directive`
    // çağrılırsa env filtresinin ÜZERİNE eklenir).
    let env = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,dtransfer_lib=debug"));
    let filter = env
        .add_directive(
            "dtransfer_lib::profiles::credentials=off"
                .parse()
                .expect("static directive"),
        )
        .add_directive(
            "dtransfer_lib::audit::redacted=off"
                .parse()
                .expect("static directive"),
        );

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_thread_names(true))
        .init();

    info!("tracing initialized");
}
