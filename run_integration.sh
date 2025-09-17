#!/bin/bash
# Simple HTTP wrapper for integration coordinator
RUST_LOG=info cargo run --bin run_integration_validation -- --port 8446
