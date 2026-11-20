//! Targeted structural payload tests; no hardware or network admission assertions.
#[path = "unit/system_payload/fixtures.rs"]
mod fixtures;
#[path = "unit/system_payload/hardware.rs"]
mod hardware;

#[path = "unit/system_payload/messaging.rs"]
mod messaging;

#[path = "unit/system_payload/transmission.rs"]
mod transmission;

#[path = "unit/system_payload/dns.rs"]
mod dns;

#[path = "unit/system_payload/dashboard.rs"]
mod dashboard;

#[path = "unit/system_payload/identity.rs"]
mod identity;

#[path = "unit/system_payload/key_rotation.rs"]
mod key_rotation;
