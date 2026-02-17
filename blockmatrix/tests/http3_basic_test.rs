// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::process::Command;
use std::thread;
use std::time::Duration;

#[test]
fn test_server_runs_without_panic() {
    // Kill any existing servers
    let _ = Command::new("pkill")
        .arg("-f")
        .arg("blockmatrix-http3")
        .output();

    thread::sleep(Duration::from_secs(1));

    // Start the server
    let mut server = Command::new("cargo")
        .args(&["run", "--bin", "blockmatrix-http3-server-minimal"])
        .spawn()
        .expect("Failed to start server");

    // Let it run for 5 seconds
    thread::sleep(Duration::from_secs(5));

    // Check if still running
    match server.try_wait() {
        Ok(None) => {
            // Process is still running, good!
            println!("Server is running successfully");
            server.kill().expect("Failed to kill server");
        }
        Ok(Some(status)) => {
            panic!("Server exited unexpectedly with status: {:?}", status);
        }
        Err(e) => {
            panic!("Error checking server status: {}", e);
        }
    }
}

#[test]
fn test_server_listens_on_udp_port() {
    // Start server in background
    let mut server = Command::new("cargo")
        .args(&["run", "--bin", "blockmatrix-http3-server-minimal"])
        .spawn()
        .expect("Failed to start server");

    // Wait for startup
    thread::sleep(Duration::from_secs(3));

    // Check UDP port
    let output = Command::new("ss")
        .args(&["-uln"])
        .output()
        .expect("Failed to run ss command");

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Clean up
    server.kill().expect("Failed to kill server");

    // Verify port 8446 is in the output
    assert!(
        output_str.contains("8446"),
        "Server not listening on UDP port 8446"
    );
}

#[test]
fn test_server_compiles_successfully() {
    let output = Command::new("cargo")
        .args(&["build", "--bin", "blockmatrix-http3-server-minimal"])
        .output()
        .expect("Failed to run cargo build");

    assert!(
        output.status.success(),
        "Server failed to compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}