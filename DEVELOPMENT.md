# CipherStream Development Guide

This document provides detailed information for developers working on the CipherStream project.

## Development Setup

### Environment

1. **Rust Toolchain:**
   - Stable Rust (1.54.0 or later) is recommended
   - Install with [rustup](https://rustup.rs/)

2. **Development Tools:**
   - Install cargo-tarpaulin for code coverage: `cargo install cargo-tarpaulin`
   - Install cargo-audit for security checks: `cargo install cargo-audit`

### Code Structure

- **src/main.rs**: Entry point and CLI handling
- **src/lib.rs**: Main library exports
- **src/crypto/**: Cryptographic operations
- **src/network/**: libp2p networking and connectivity
- **src/protocol/**: Message definitions and protocol logic
- **src/file_transfer/**: File transmission logic
- **src/discovery/**: Peer discovery mechanisms
- **src/utils/**: Utility functions and helpers
- **tests/**: Integration and specific test modules

## Testing Strategy

CipherStream uses a comprehensive testing approach with several categories of tests:

### Unit Tests

Unit tests are defined within the source files using Rust's standard `#[cfg(test)]` module pattern:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test code
    }
}
```

### Integration Tests

Integration tests are located in the `tests/` directory and test multiple components together. These tests are more complex and may have special requirements:

- Network tests require connectivity and can occasionally fail due to port conflicts
- Some tests are marked as `#[ignore]` by default to prevent them from breaking CI pipelines
- Use `cargo test -- --ignored` to run these tests explicitly

### Performance Tests

Performance benchmarks measure the efficiency of critical operations like:

- Encryption/decryption
- Signing/verification
- File hashing

These tests output timing information to help identify performance regressions.

## Running Tests

The `test.sh` script provides a unified interface for running different test categories:

```bash
# Run all unit tests
./test.sh unit

# Run integration tests (may hang or fail)
./test.sh integration

# Run performance benchmarks
./test.sh benchmarks

# Run test coverage (requires cargo-tarpaulin)
./test.sh coverage

# Run all tests
./test.sh all
```

### Addressing Common Test Issues

#### Port Conflicts

Integration tests that use network connections may fail due to port conflicts with the error:
`Address already in use (os error 48)`. Solutions include:

1. Using ephemeral ports (port 0) which allow the OS to assign available ports
2. Add unique port offsets based on test names
3. Ensuring cleanup of previous test runs with `pkill -f cipherstream`

#### Test Timeouts

Complex network tests may sometimes hang. We use the following strategies:

1. Adding explicit timeouts
2. Marking flaky tests as `#[ignore]`
3. Running tests with isolation using `--test test_name`

## Continuous Integration

Our CI pipeline runs the following checks:

1. **Compilation**: `cargo build`
2. **Unit Tests**: `cargo test --lib`
3. **Linting**: `cargo clippy`
4. **Formatting**: `cargo fmt --check`
5. **Security Check**: `cargo audit`

Integration tests are not run on CI due to potential network issues.

## Working with libp2p

CipherStream heavily uses libp2p for peer-to-peer networking. Key concepts include:

### Multiaddress

A self-describing network address format used by libp2p. Examples:

- `/ip4/127.0.0.1/tcp/8000` - IPv4 address with TCP port 8000
- `/ip4/127.0.0.1/tcp/8000/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N` - With peer ID

### PeerId

A unique identifier for each node on the network, derived from the node's public key.

### Network Behavior

The `Behavior` struct in `src/network/mod.rs` defines the network capabilities of our nodes including:
- Kademlia DHT for peer discovery
- mDNS for local network discovery
- Request-response protocols for file transfer messages

## Adding New Features

When adding new features:

1. Start with tests that define the expected behavior
2. Implement the feature with well-documented code
3. Ensure both unit and integration tests pass
4. Document the feature in relevant README sections

## Release Process

1. Update the version in `Cargo.toml`
2. Update the CHANGELOG.md file
3. Create a git tag for the release version
4. Build release binaries for target platforms
5. Publish the release 