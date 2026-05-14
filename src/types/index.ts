/**
 * Rust tarafıyla paylaşılan JSON-projeksiyon tipleri.
 *
 * Bu tipler Tauri IPC üzerinden gelen payload'ları aynalar. Rust tarafında
 * `#[derive(Serialize)]` ile yazılan struct'ların yapısı buraya manuel
 * senkronlanır — koddan otomatik üretim Faz 2'de (ts-rs veya benzeri).
 */

export type Protocol = "sftp" | "s3" | "webdav" | "local";

export type TransferState =
  | "queued"
  | "active"
  | "verifying"
  | "finalizing"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled"
  | "skipped";

export type TransferDirection = "upload" | "download";

export interface ProgressPayload {
  transferId: string;
  bytesDone: number;
  bytesTotal: number;
  speedBps: number;
  etaSeconds: number | null;
  state: TransferState;
}

export interface EngineLogEvent {
  level: "trace" | "debug" | "info" | "warn" | "error";
  target: string;
  message: string;
  timestamp: number;
}

/** Rust `commands::EngineStatus` camelCase projeksiyonu. */
export interface EngineStatus {
  running: boolean;
  cancelled: boolean;
  eventSubscribers: number;
}

/** Rust `protocols::types::AdapterCapabilities` camelCase projeksiyonu. */
export interface AdapterCapabilities {
  supportsByteRange: boolean;
  supportsRemoteChecksum: boolean;
  supportsServerSideRename: boolean;
  supportsSymlinks: boolean;
  supportsResume: boolean;
  supportsMultipart: boolean;
  maxParallelSessions: number;
}

/** Rust `commands::LocalTransferRequest` camelCase projeksiyonu. */
export interface LocalTransferRequest {
  root: string;
  source: string;
  destination: string;
}

/** Rust `commands::LocalTransferReport` camelCase projeksiyonu. */
export interface LocalTransferReport {
  transferId: string;
  bytesTransferred: number;
  durationMs: number;
  avgSpeedBps: number;
}

/** Rust `settings::ChecksumAlgo` camelCase projeksiyonu. */
export type ChecksumAlgo = "none" | "sha256" | "xxHash3";

/** Rust `settings::FsyncPolicy` camelCase projeksiyonu. */
export type FsyncPolicy = "none" | "dataOnly" | "full";

/** Rust `settings::AppSettings` camelCase projeksiyonu. */
export interface AppSettings {
  schemaVersion: number;
  defaultDownloadDir: string | null;
  maxConcurrentTransfers: number;
  defaultChunkSizeMb: number;
  defaultMaxInflightMb: number;
  bandwidthLimitBps: number | null;
  verifyChecksum: ChecksumAlgo;
  fsyncPolicy: FsyncPolicy;
  autoUpdate: boolean;
  telemetry: boolean;
}

/** Rust `settings::AppSettingsPatch` — UI sadece değiştirdiği alanı yollar.
 *  `null` değer açıkça unset etmek anlamına gelir (`Option<Option<T>>`'un dış
 *  Some(None) projeksiyonu); alanın atlanması no-op. */
export interface AppSettingsPatch {
  defaultDownloadDir?: string | null;
  maxConcurrentTransfers?: number;
  defaultChunkSizeMb?: number;
  defaultMaxInflightMb?: number;
  bandwidthLimitBps?: number | null;
  verifyChecksum?: ChecksumAlgo;
  fsyncPolicy?: FsyncPolicy;
  autoUpdate?: boolean;
  telemetry?: boolean;
}

/** Rust `commands::LocalEntry` camelCase projeksiyonu — UI local browser. */
export interface LocalEntry {
  name: string;
  path: string;
  kind: "file" | "directory" | "symlink" | "other";
  size: number | null;
  modifiedUnixMs: number | null;
  isHidden: boolean;
}

/** Rust `commands::ListLocalDirResponse` camelCase projeksiyonu. */
export interface ListLocalDirResponse {
  path: string;
  parent: string | null;
  entries: LocalEntry[];
}

/** Rust `commands::ListLocalDirRequest` camelCase projeksiyonu. */
export interface ListLocalDirRequest {
  path: string;
  includeHidden: boolean;
}

// ----------------------------------------------------------------------------
// Remote browser (Bölüm 19 — Faz 4 ConnectionManager üzerinden)
// ----------------------------------------------------------------------------

/** Rust `commands::RemoteEntryDto` camelCase projeksiyonu. */
export interface RemoteEntryDto {
  name: string;
  path: string;
  kind: "file" | "directory" | "symlink" | "other";
  size: number | null;
  modifiedUnixMs: number | null;
  isHidden: boolean;
}

/** Rust `commands::ListRemoteDirResponse` camelCase projeksiyonu. */
export interface ListRemoteDirResponse {
  path: string;
  parent: string | null;
  entries: RemoteEntryDto[];
}

/** Rust `commands::ListRemoteDirRequest` camelCase projeksiyonu. */
export interface ListRemoteDirRequest {
  profileId: string;
  path: string;
  includeHidden: boolean;
}

/** Rust `commands::EnqueueTransferRequest`. */
export interface EnqueueTransferRequest {
  profileId: string;
  localPath: string;
  remotePath: string;
  bytesTotal?: number;
}

/** Rust `commands::EnqueueTransferResponse`. */
export interface EnqueueTransferResponse {
  transferId: string;
}

/** Rust `commands::TransferDto` camelCase projeksiyonu — queue panel satırı. */
export interface TransferDto {
  id: string;
  profileId: string;
  direction: TransferDirection;
  state: TransferState;
  priority: number;
  localPath: string;
  remotePath: string;
  bytesTotal: number;
  bytesDone: number;
  chunkSize: number;
  retryCount: number;
  lastError: string | null;
  createdAt: string;
  updatedAt: string;
  startedAt: string | null;
  completedAt: string | null;
}

/** Rust `errors::WireError` camelCase projeksiyonu. */
export type ErrorCategory =
  | "network"
  | "auth"
  | "permission"
  | "notFound"
  | "conflict"
  | "rateLimit"
  | "serverError"
  | "integrity"
  | "cancelled"
  | "unknown";

export interface WireError {
  category: ErrorCategory;
  suggestedAction: string;
  i18nKey: string;
  message: string;
}

// ----------------------------------------------------------------------------
// ConnectionProfile (Bölüm 25)
// ----------------------------------------------------------------------------

/** Rust `profiles::ProfileProtocol` camelCase projeksiyonu. */
export type ProfileProtocol = "local" | "sftp" | "s3" | "webdav";

/** Rust `profiles::AuthMethod` camelCase projeksiyonu. */
export type AuthMethod = "none" | "password" | "publicKey";

/**
 * Rust `profiles::ConnectionProfile` camelCase projeksiyonu.
 *
 * Sırlar (parola, private-key passphrase) bu yapıda YOKTUR — OS keystore
 * üzerinden yönetilir. IPC çağrılarında secret ayrı bir parametre olarak
 * gönderilir / silinir.
 */
export interface ConnectionProfile {
  id: string;
  name: string;
  protocol: ProfileProtocol;
  host: string | null;
  port: number | null;
  username: string | null;
  remoteRoot: string | null;
  localRoot: string | null;
  authMethod: AuthMethod;
  optionsJson: string;
  createdAt: string;
  updatedAt: string;
}

/**
 * Rust `events::EngineEvent` discriminated-union projeksiyonu (Bölüm 33).
 *
 * Rust tarafı `#[serde(tag = "type", rename_all = "camelCase")]` ile tag-ed
 * JSON üretir; bu union onun tip-güvenli TS karşılığıdır.
 */
export type EngineEvent =
  | {
      type: "transferProgress";
      transferId: string;
      bytesDone: number;
      bytesTotal: number;
      speedBps: number;
      etaSecs: number | null;
    }
  | {
      type: "transferStateChanged";
      transferId: string;
      oldState: TransferState;
      newState: TransferState;
    }
  | {
      type: "transferCompleted";
      transferId: string;
      checksum: string;
      durationMs: number;
    }
  | {
      type: "transferFailed";
      transferId: string;
      error: WireError;
      retryInMs: number | null;
    }
  | { type: "rateLimited"; profileId: string; retryAfterSecs: number }
  | { type: "connectionLost"; profileId: string }
  | { type: "connectionRestored"; profileId: string }
  | { type: "queueRecovered"; restoredCount: number }
  | { type: "queueDrained" }
  | { type: "appShutdownInitiated" }
  | { type: "diagnosticsFlushed"; path: string };
