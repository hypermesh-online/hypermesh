//! Fake operations validate actual production branching, not kernel use.
use super::{attach_then_socket, AttachmentOps};

struct Operations {
    attach_ok: bool,
    socket_ok: bool,
    kernel_backed: bool,
    calls: Vec<&'static str>,
}

impl AttachmentOps for Operations {
    type Socket = bool;

    fn attach(&mut self, _: &str) -> Result<(), String> {
        self.calls.push("attach");
        self.attach_ok
            .then_some(())
            .ok_or_else(|| "attach refused".into())
    }

    fn create_socket(&mut self, _: &str) -> Result<bool, String> {
        self.calls.push("socket");
        self.socket_ok
            .then_some(self.kernel_backed)
            .ok_or_else(|| "socket unavailable".into())
    }
    fn kernel_backed(socket: &Self::Socket) -> bool {
        *socket
    }
}

fn operations(attach_ok: bool, socket_ok: bool) -> Operations {
    Operations {
        attach_ok,
        socket_ok,
        kernel_backed: true,
        calls: Vec::new(),
    }
}

#[test]
fn failed_attachment_never_attempts_socket_or_reports_attached() {
    let mut ops = operations(false, true);
    assert_eq!(
        attach_then_socket(&mut ops, "unit"),
        Err("attach refused".into())
    );
    assert_eq!(ops.calls, ["attach"]);
}

#[test]
fn successful_attachment_precedes_socket_creation() {
    let mut ops = operations(true, true);
    assert_eq!(attach_then_socket(&mut ops, "unit"), Ok(Some(true)));
    assert_eq!(ops.calls, ["attach", "socket"]);
}

#[test]
fn attached_without_socket_retains_standard_io_fallback() {
    let mut ops = operations(true, false);
    assert_eq!(attach_then_socket(&mut ops, "unit"), Ok(None));
    assert_eq!(ops.calls, ["attach", "socket"]);
}

#[test]
fn explicit_retry_uses_a_new_attachment_attempt() {
    let mut ops = operations(false, true);
    assert!(attach_then_socket(&mut ops, "unit").is_err());
    ops.attach_ok = true;
    assert_eq!(attach_then_socket(&mut ops, "unit"), Ok(Some(true)));
    assert_eq!(ops.calls, ["attach", "attach", "socket"]);
}

#[test]
fn software_only_socket_never_installed_as_acceleration() {
    let mut ops = operations(true, true);
    ops.kernel_backed = false;
    assert_eq!(attach_then_socket(&mut ops, "unit"), Ok(None));
    assert_eq!(ops.calls, ["attach", "socket"]);
}
