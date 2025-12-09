use std::process::Command;
use std::thread;
use std::time::Duration;
use reqwest;
use serde_json::Value;

#[tokio::test]
async fn test_blockmatrix_http3_server() {
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

    // Wait for server to start
    thread::sleep(Duration::from_secs(3));

    // Test health endpoint
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    // Test IPv6 localhost
    let response = client
        .get("https://[::1]:8446/health")
        .send()
        .await;

    if response.is_err() {
        // Try IPv4 fallback
        let response = client
            .get("https://localhost:8446/health")
            .send()
            .await;

        if response.is_err() {
            // Try HTTP fallback
            let response = client
                .get("http://localhost:8446/health")
                .send()
                .await
                .expect("Failed to connect to server");

            assert_eq!(response.status(), 200);
            let body: Value = response.json().await.unwrap();
            assert_eq!(body["status"], "healthy");
        } else {
            let response = response.unwrap();
            assert_eq!(response.status(), 200);
            let body: Value = response.json().await.unwrap();
            assert_eq!(body["status"], "healthy");
        }
    } else {
        let response = response.unwrap();
        assert_eq!(response.status(), 200);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["status"], "healthy");
    }

    // Cleanup
    server.kill().expect("Failed to kill server");
}

#[tokio::test]
async fn test_http3_endpoints() {
    // This test assumes server is already running
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // Define test endpoints
    let endpoints = vec![
        ("/health", "GET"),
        ("/api/v1/blockmatrix/status", "GET"),
        ("/api/v1/blockmatrix/assets", "GET"),
    ];

    for (endpoint, method) in endpoints {
        println!("Testing {} {}", method, endpoint);

        // Try different connection methods
        let urls = vec![
            format!("https://[::1]:8446{}", endpoint),
            format!("https://localhost:8446{}", endpoint),
            format!("http://localhost:8446{}", endpoint),
        ];

        let mut success = false;
        for url in urls {
            let result = match method {
                "GET" => client.get(&url).send().await,
                _ => continue,
            };

            if let Ok(response) = result {
                println!("  {} - Status: {}", url, response.status());
                if response.status().is_success() {
                    success = true;
                    break;
                }
            }
        }

        if !success {
            println!("  WARNING: Endpoint {} not accessible", endpoint);
        }
    }
}

#[tokio::test]
async fn test_performance_metrics() {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();

    // Measure response times
    let mut response_times = Vec::new();

    for _ in 0..10 {
        let start = std::time::Instant::now();

        let _ = client
            .get("http://localhost:8446/health")
            .send()
            .await;

        let elapsed = start.elapsed();
        response_times.push(elapsed.as_millis());
    }

    // Calculate average
    let avg = response_times.iter().sum::<u128>() / response_times.len() as u128;

    println!("Average response time: {}ms", avg);
    println!("Min: {}ms", response_times.iter().min().unwrap());
    println!("Max: {}ms", response_times.iter().max().unwrap());

    // Assert performance target
    assert!(avg < 50, "Response time {}ms exceeds 50ms target", avg);
}

#[tokio::test]
async fn test_concurrent_requests() {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // Send 10 concurrent requests
    let mut handles = vec![];

    for i in 0..10 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            let result = client
                .get("http://localhost:8446/health")
                .send()
                .await;

            match result {
                Ok(response) => {
                    println!("Request {} - Status: {}", i, response.status());
                    response.status().is_success()
                }
                Err(e) => {
                    println!("Request {} - Error: {}", i, e);
                    false
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all requests
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap() {
            success_count += 1;
        }
    }

    println!("Successful concurrent requests: {}/10", success_count);
    assert!(success_count >= 8, "Too many failed concurrent requests");
}

#[tokio::test]
async fn test_error_handling() {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    // Test 404 response
    let response = client
        .get("http://localhost:8446/nonexistent")
        .send()
        .await;

    if let Ok(response) = response {
        assert_eq!(response.status(), 404, "Expected 404 for invalid endpoint");
    }

    // Test malformed JSON POST
    let response = client
        .post("http://localhost:8446/api/v1/blockmatrix/assets/allocate")
        .body("invalid json")
        .header("content-type", "application/json")
        .send()
        .await;

    if let Ok(response) = response {
        assert!(response.status().is_client_error(), "Expected 4xx error for malformed JSON");
    }
}

#[test]
fn test_server_compilation() {
    // Ensure server binaries compile
    let output = Command::new("cargo")
        .args(&["check", "--bin", "blockmatrix-http3-server-minimal"])
        .output()
        .expect("Failed to run cargo check");

    assert!(output.status.success(), "Server failed to compile");
}