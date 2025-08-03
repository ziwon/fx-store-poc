# Justfile for fx-store

# Development build
build:
    cargo build

# Optimized release build (use for performance testing)
release:
    RUSTFLAGS="-C target-cpu=native" cargo build --release

# Run tests
test:
    cargo test

# Run benchmarks (when implemented)
bench:
    cargo bench

# Check for compilation errors without building
check:
    cargo check

# Run the program
run:
    cargo run

# Run with release optimizations
run-release:
    RUSTFLAGS="-C target-cpu=native" cargo run --release

# Clean build artifacts
clean:
    cargo clean

# Format code
fmt:
    cargo fmt

# Run clippy linter
clippy:
    cargo clippy

# Check formatting
fmt-check:
    cargo fmt --check

# All checks (format, clippy, test)
ci: fmt-check clippy test
