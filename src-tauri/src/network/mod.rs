//! Network resilience + proxy katmanı — Bölüm 37 + 38.
//!
//! - [`keepalive::SshKeepalive`] (38.1) — russh ve TCP-SO_KEEPALIVE
//!   parametreleri. Default 30sn interval, 3 packet fail → drop.
//! - [`proxy::ProxyConfig`] (37.1) — HTTP / HTTPS / SOCKS5 proxy tarifi.
//! - [`proxy::bypass_matches`] (37.3) — glob pattern host eşleştirme; `*.local`,
//!   `10.0.*`, `localhost` gibi kuralları destekler.

pub mod keepalive;
pub mod proxy;

pub use keepalive::{ConnectStrategy, SshKeepalive};
pub use proxy::{bypass_matches, ProxyConfig, ProxyKind, ProxySource};
