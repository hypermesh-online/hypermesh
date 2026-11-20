// Copyright 2026 Hypermesh Foundation. All rights reserved.
//! Production attachment ordering, independent of kernel capability detection.
//! Successful operations are backend results, not independent traffic evidence.

pub(crate) trait AttachmentOps {
    type Socket;
    fn attach(&mut self, interface: &str) -> Result<(), String>;
    fn create_socket(&mut self, interface: &str) -> Result<Self::Socket, String>;
    fn kernel_backed(socket: &Self::Socket) -> bool;
}

/// Never create a socket after an explicit attachment failure.
/// An attached program without AF_XDP keeps legitimate standard-I/O fallback.
pub(crate) fn attach_then_socket<T: AttachmentOps>(
    operations: &mut T,
    interface: &str,
) -> Result<Option<T::Socket>, String> {
    operations.attach(interface)?;
    Ok(operations
        .create_socket(interface)
        .ok()
        .filter(T::kernel_backed))
}

#[cfg(test)]
#[path = "tests/kernel_attachment.rs"]
mod tests;
