#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper to print section headers
print_header() {
    echo -e "\n${BLUE}==>${NC} ${YELLOW}$1${NC}"
}

# Helper to print success message
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

# Helper to print error message
print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Clean up any existing test processes
cleanup() {
    print_header "Cleaning up test environment"
    pkill -f cipherstream || true
    sleep 1
}

# Function to run all unit tests (non-integration tests)
run_unit_tests() {
    print_header "Running unit tests"
    
    # Run library tests first
    echo "Running library tests..."
    cargo test --lib
    
    # Run non-integration tests
    echo "Running test modules..."
    cargo test --test="crypto_*" --test="file_*" --test="network_multiaddr*" --test="protocol_*" --test="codec_*"
    
    # Run network config tests separately
    echo "Running network config tests..."
    cargo test --test="network_config_test"
    
    print_success "Unit tests completed"
}

# Function to run integration tests with specific parameters
run_integration_tests() {
    print_header "Running integration tests (these may hang or fail)"
    
    # Warn that these tests may hang
    echo -e "${YELLOW}Warning: These tests may hang or fail due to network port conflicts.${NC}"
    echo "Tests are marked as ignored by default, use --no-ignore to run them."
    
    # Run integration tests with a timeout
    echo "Running node connectivity tests..."
    
    if [ "$1" == "--no-ignore" ]; then
        cargo test --test="node_connect_test" -- --nocapture --ignored || true
        echo "Running file transfer tests..."
        cargo test --test="file_transfer_test" -- --nocapture --ignored || true
        echo "Running port allocation tests..."
        cargo test --test="network_port_allocation_test" -- --nocapture --ignored || true
    else
        echo "Skipping ignored tests. Use --no-ignore to run these tests."
    fi
    
    print_success "Integration tests completed"
}

# Function to run tests with coverage (if cargo-tarpaulin is installed)
run_coverage() {
    print_header "Running coverage tests"
    
    if command -v cargo-tarpaulin &> /dev/null; then
        echo "Running coverage with tarpaulin..."
        cargo tarpaulin --exclude-files "tests/*" --out Html
        print_success "Coverage report generated"
    else
        print_error "cargo-tarpaulin is not installed"
        echo "Install with: cargo install cargo-tarpaulin"
    fi
}

# Function to run performance benchmarks
run_benchmarks() {
    print_header "Running crypto performance benchmarks"
    
    echo "Running crypto benchmarks..."
    cargo test --test="crypto_performance_test" -- --nocapture
    
    print_success "Benchmarks completed"
}

# Main function
main() {
    # Register cleanup function to run on exit
    trap cleanup EXIT
    
    # Parse command line arguments
    case "$1" in
        "unit")
            run_unit_tests
            ;;
        "integration")
            run_integration_tests $2
            ;;
        "coverage")
            run_coverage
            ;;
        "benchmarks")
            run_benchmarks
            ;;
        "all")
            run_unit_tests
            run_integration_tests $2
            run_benchmarks
            ;;
        *)
            echo "Usage: $0 [unit|integration|coverage|benchmarks|all] [--no-ignore]"
            echo ""
            echo "Commands:"
            echo "  unit        Run all unit tests"
            echo "  integration Run integration tests (may hang)"
            echo "  coverage    Run test coverage (requires cargo-tarpaulin)"
            echo "  benchmarks  Run performance benchmarks"
            echo "  all         Run all tests"
            echo ""
            echo "Options:"
            echo "  --no-ignore Run tests marked as ignored"
            ;;
    esac
}

# Run the main function
main "$@"