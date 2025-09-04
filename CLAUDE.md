# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Wasmtime is a standalone runtime for WebAssembly, supporting both WebAssembly modules and the Component Model. It's part of the Bytecode Alliance and provides a safe, efficient, and ergonomic embedding API for running WebAssembly in various environments.

## Build and Development Commands

### Build Commands
```bash
# Build all packages (in release mode for better performance)
cargo build --release

# Build the Wasmtime CLI specifically
cargo build --package wasmtime-cli

# Build with all features enabled
cargo build --all-features

# Build for testing with additional features
cargo build --features "default winch pulley all-arch call-hook memory-protection-keys component-model-async"
```

### Test Commands
```bash
# Run all tests
cargo test

# Run tests for a specific package
cargo test --package wasmtime

# Run a specific test
cargo test test_name

# Run tests with all features
cargo test --all-features

# Run WebAssembly spec tests
cargo test --test wast
```

### Format and Lint
```bash
# Format all Rust code
cargo +stable fmt --all

# Check formatting without making changes
cargo +stable fmt --all -- --check

# Run clippy linting
cargo clippy --all-targets --all-features
```

### CI-specific Commands
```bash
# Build source tarball with vendored dependencies
./ci/build-src-tarball.sh

# Print current version
./ci/print-current-version.sh
```

## Architecture Overview

### Project Structure
- **`crates/wasmtime/`**: Core Wasmtime runtime library
  - Contains the main embedding API for WebAssembly modules
  - Key types: `Engine`, `Store`, `Module`, `Instance`, `Func`, `Memory`, `Table`
  
- **`crates/wasmtime/src/component/`**: Component Model implementation
  - Mirrors core WebAssembly API structure for components
  - Key types: `Component`, `component::Func`, `component::Instance`

- **`crates/cranelift/`**: Cranelift code generator used by Wasmtime
  - Optimizing compiler backend for WebAssembly
  - Contains instruction selection, register allocation, and machine code generation

- **`crates/environ/`**: Environment abstraction layer
  - Module translation and compilation environment
  - Shared between Cranelift and Winch backends

- **`crates/wasi*/`**: WASI (WebAssembly System Interface) implementations
  - `wasi`: Main WASI preview2 implementation
  - `wasi-common`: WASI preview1 implementation
  - `wasi-http`: HTTP support for WASI
  - `wasi-nn`: Neural network support
  - `wasi-threads`: Threading support

- **`crates/winch/`**: Alternative baseline compiler (faster compilation, slower runtime)

- **`pulley/`**: Portable bytecode interpreter for WebAssembly

### Key Architectural Concepts

1. **Engine**: Global compilation and runtime environment, shared across threads. Configured via `Config` with many tuning options.

2. **Store**: Container for WebAssembly runtime state, including instances, memories, tables, and host data. Each Store has a type parameter for host-specific data.

3. **Linker**: Provides host functionality to WebAssembly guests, manages imports/exports between modules.

4. **Module Compilation Flow**:
   - WAT/WASM → Parse → Validate → Translate to IR → Compile (Cranelift/Winch) → Machine Code
   - Supports both JIT and ahead-of-time compilation

5. **Component Model**: Higher-level abstraction over modules with interface types and resource management.

## Working with Tests

The test suite is organized in several categories:

- **`tests/all/`**: Integration tests for the Wasmtime API
- **`tests/spec_testsuite/`**: Official WebAssembly spec tests
- **`tests/wasi_testsuite/`**: WASI-specific tests
- **`tests/disas/`**: Disassembly tests for generated code

Tests can include special markers in commit messages for CI:
- `prtest:full` - Run full CI suite
- `prtest:debug` - Run with debug/DWARF tests
- `prtest:platform-checks` - Run platform-specific checks

## Dependencies and Features

Major features controlled via Cargo features:
- `component-model`: Enable Component Model support
- `cranelift`: Use Cranelift compiler (default)
- `winch`: Use Winch baseline compiler
- `pulley`: Use Pulley interpreter
- `wasi-*`: Various WASI features
- `async`: Async support for components and WASI

The project uses workspace dependencies defined in the root `Cargo.toml` for consistent versioning.