# Testing Standards

**Version**: 1.0
**Date**: 2025-12-02
**Purpose**: Ensure tests accurately reflect implementation status and provide real confidence

## Core Principles

1. **Tests must match reality** - Never test stub implementations without marking them
2. **Honest feedback** - Tests should fail when features don't work
3. **Clear documentation** - Every test should indicate what it's actually testing
4. **No false confidence** - Better to have no test than a misleading test

## Test Categories

### 1. Unit Tests
**Purpose**: Test individual functions/methods in isolation
**Location**: `src/**/*.rs` in `#[cfg(test)]` modules
**Requirements**:
- Test actual behavior, not just return values
- Mock external dependencies appropriately
- Cover edge cases and error conditions

### 2. Integration Tests
**Purpose**: Test component interactions
**Location**: `tests/` directory
**Requirements**:
- Test real component integration
- Use actual implementations, not mocks
- Verify data flow between components

### 3. End-to-End Tests
**Purpose**: Test complete user workflows
**Location**: `tests/e2e/`
**Requirements**:
- Test from user perspective
- Cover critical user journeys
- Include both success and failure paths

## Required Test Patterns

### Testing Real Implementations

```rust
#[test]
fn test_real_feature() {
    // Setup
    let component = RealComponent::new();

    // Execute
    let result = component.do_something();

    // Verify actual behavior
    assert_eq!(result.value, expected_value);
    assert!(result.side_effect_occurred());
    // Don't just check Ok(())!
}
```

### Testing Stub Implementations

```rust
#[test]
#[ignore = "Feature not implemented - see STUB_INVENTORY.md"]
fn test_future_feature() {
    // This test documents expected behavior
    // Will be enabled when feature is implemented
    let component = FutureComponent::new();
    let result = component.not_yet_implemented();
    assert_eq!(result, expected_behavior);
}
```

### Testing Partial Implementations

```rust
#[test]
fn test_partial_feature() {
    // Test what works
    let component = PartialComponent::new();
    assert!(component.implemented_part().is_ok());

    // Document what doesn't work
    // Note: advanced_feature() is stubbed - see TODO in component.rs
}
```

## Forbidden Patterns

### ❌ Testing Only Return Status

```rust
// BAD: Provides no real validation
#[test]
fn test_bad() {
    let result = do_something();
    assert!(result.is_ok()); // This tells us nothing!
}
```

### ❌ Testing Stub Without Marking

```rust
// BAD: Tests stub implementation without indicating it
#[test]
fn test_consensus() {
    // This always passes because validate() always returns true
    let result = validator.validate();
    assert!(result);
}
```

### ❌ Unrealistic Mock Data

```rust
// BAD: Mock data doesn't represent reality
#[test]
fn test_performance() {
    let fake_data = vec![1, 2, 3]; // Real data would be MB/GB
    let result = process(fake_data);
    assert!(result.is_fast()); // Meaningless with toy data
}
```

## Test Documentation Requirements

### File Header

Every test file must include:

```rust
//! Test module for [Component Name]
//!
//! Implementation Status: [Percentage]% complete
//! - Working: [List what works]
//! - Stubbed: [List what's stubbed]
//! - Missing: [List what's not implemented]
```

### Test Function Documentation

```rust
/// Tests [specific functionality]
///
/// **Implementation Status**: [Full/Partial/Stub]
/// **Dependencies**: [List any dependencies]
/// **Known Issues**: [List any issues]
#[test]
fn test_specific_feature() {
    // ...
}
```

## Ignore Attribute Usage

Use `#[ignore]` with descriptive messages:

```rust
#[test]
#[ignore = "Container runtime not implemented - see STUB_INVENTORY.md"]
fn test_container_lifecycle() { }

#[test]
#[ignore = "Requires multi-node support - currently single-node only"]
fn test_distributed_consensus() { }

#[test]
#[ignore = "Performance test - run with --ignored --release"]
fn bench_large_dataset() { }
```

## Test Coverage Requirements

### For New Features

1. **Before Implementation**
   - Write tests marked with `#[ignore]`
   - Document expected behavior
   - Define success criteria

2. **During Implementation**
   - Remove `#[ignore]` as features are completed
   - Update tests to match actual implementation
   - Add edge case tests

3. **After Implementation**
   - All tests must pass
   - Coverage must include happy path and error cases
   - Performance characteristics documented

### For Existing Code

1. **Real implementations**: Must have at least one test
2. **Stub implementations**: Must be marked with `#[ignore]`
3. **Deprecated code**: Tests can be removed

## Running Tests

### Standard Test Run
```bash
# Run all non-ignored tests
cargo test

# Run specific test suite
cargo test --package stoq

# Run with output
cargo test -- --nocapture
```

### Including Ignored Tests
```bash
# Run ignored tests only
cargo test -- --ignored

# Run all tests including ignored
cargo test -- --include-ignored
```

### Performance Tests
```bash
# Run benchmarks
cargo bench

# Run performance tests in release mode
cargo test --release -- --ignored performance
```

## Test Maintenance

### Quarterly Review

Every 3 months, review:
1. Which ignored tests can be enabled (features implemented)
2. Which tests need updating (implementation changed)
3. Which tests should be removed (features deprecated)
4. Coverage gaps that need new tests

### When Implementation Changes

1. **Update tests immediately** when implementation changes
2. **Mark as ignored** if implementation is removed/stubbed
3. **Document** the change in test comments
4. **Update** TEST_COVERAGE_REPORT.md

## CI/CD Requirements

### Pre-commit Checks
- All non-ignored tests must pass
- No new tests without documentation
- No unmarked stub tests

### PR Requirements
- New features must include tests
- Modified code must maintain or improve coverage
- Ignored tests must have descriptive reasons

### Release Criteria
- 100% of non-ignored tests passing
- No critical paths without tests
- Performance tests show no regression

## Common Test Scenarios

### Testing Async Code
```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Testing Error Conditions
```rust
#[test]
fn test_error_handling() {
    let result = operation_that_fails();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Expected);
}
```

### Testing with Timeouts
```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_with_timeout() {
    let result = timeout(Duration::from_secs(5), slow_operation()).await;
    assert!(result.is_ok());
}
```

## Best Practices

1. **Test names should be descriptive**
   - Good: `test_cpu_allocation_exceeds_available_cores`
   - Bad: `test_cpu_fail`

2. **One assertion per test concept**
   - Group related assertions
   - Separate different concepts into different tests

3. **Use test fixtures for common setup**
   - Create helper functions for repeated setup
   - Use `once_cell` for expensive shared setup

4. **Clean up after tests**
   - Remove temporary files
   - Close network connections
   - Reset global state

5. **Make tests deterministic**
   - Avoid random data without seeding
   - Mock time-dependent operations
   - Control concurrency in tests

## Appendix: Test Attributes

| Attribute | Purpose | Example |
|-----------|---------|---------|
| `#[test]` | Mark function as test | Basic unit test |
| `#[tokio::test]` | Async test with tokio | Testing async functions |
| `#[ignore]` | Skip test by default | Unimplemented features |
| `#[should_panic]` | Expect panic | Testing panic conditions |
| `#[cfg(test)]` | Test-only module | Test helpers and mocks |

## References

- [STUB_INVENTORY.md](./STUB_INVENTORY.md) - List of all stub implementations
- [TEST_COVERAGE_REPORT.md](./TEST_COVERAGE_REPORT.md) - Current test coverage analysis
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)