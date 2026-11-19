//! Targeted structural payload tests; no hardware or network admission assertions.
#[path = "unit/system_payload/fixtures.rs"]
mod fixtures;
#[path = "unit/system_payload/hardware.rs"]
mod hardware;

#[path = "unit/system_payload/messaging.rs"]
mod messaging;

#[path = "unit/system_payload/transmission.rs"]
mod transmission;
