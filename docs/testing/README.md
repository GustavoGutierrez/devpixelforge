# Testing Documentation

Complete testing suite documentation for **devpixelforge**.

---

## 📚 Table of Contents

| Document | Description | Audience |
|----------|-------------|----------|
| [`01-architecture.md`](01-architecture.md) | Test architecture and structure | Architects, Tech Leads |
| [`02-rust-tests.md`](02-rust-tests.md) | Rust tests guide (unit + integration) | Rust Developers |
| [`03-go-tests.md`](03-go-tests.md) | Go bridge tests guide | Go Developers |
| [`04-running-and-ci.md`](04-running-and-ci.md) | Execution, CI/CD and troubleshooting | DevOps, CI Engineers |

---

## 🚀 Quick Start

### Run All Tests

```bash
# Basic test (Rust caps verification)
make test

# Full Rust tests
cd dpf && cargo test

# Go tests
cd go-bridge && go test -v
```

### Run by Component

```bash
# Rust unit tests only
cd dpf && cargo test --lib

# Rust integration tests only
cd dpf && cargo test --test integration_tests

# Go tests (requires compiled Rust binary)
cd go-bridge && go test -v
```

---

## 📊 Test Summary

| Language | Type | Count | Location |
|----------|------|-------|----------|
| **Rust** | Unit | 280+ | `src/*/tests` (inline) |
| **Rust** | Integration | 20+ | `tests/integration_tests.rs` |
| **Go** | Unit + Integration | 16+ | `dpf_test.go` |
| **Total** | - | **316+** | - |

---

## 🏗️ Test Architecture

```
docs/testing/
├── 01-architecture.md          # Architecture
├── 02-rust-tests.md            # Rust guide
├── 03-go-tests.md              # Go guide
├── 04-running-and-ci.md        # CI/CD
└── README.md                   # This file

dpf/
├── src/
│   └── operations/              # Unit tests inline
├── tests/
│   └── integration_tests.rs    # Integration tests
├── examples/
│   └── gen_fixtures.rs         # Fixture generator
└── test_fixtures/              # Test images

go-bridge/
└── dpf_test.go                 # Go client tests
```

---

## 🔧 Test Fixtures

Fixtures are auto-generated:

```bash
cd dpf && cargo run --example gen_fixtures
```

**Available fixtures:**
- `sample.png` - RGBA 100x100 gradient
- `sample.jpg` - JPEG 100x100
- `sample.svg` - SVG vector 100x100
- `sample_transparent.png` - PNG with alpha
- `large.png` - 1000x1000 PNG
- `solid_red.png` / `solid_blue.png` - Solid colors
- `corrupt/bad.png` - Corrupt file for error tests

---

## 📝 Conventions

### Rust
- Inline tests with `#[cfg(test)] mod tests`
- Naming: `test_<function>_<case>`
- Use `tempfile::TempDir` for temp directories

### Go
- Separate `*_test.go` files
- Naming: `Test<Component><Case>`
- Use `t.TempDir()` for temp directories
- Integration tests `t.Skip()` if no binary

---

## 🐛 Troubleshooting

### "Binary not found" in Go tests
```bash
cd dpf && cargo build
cd go-bridge && go test -v
```

### Slow tests
```bash
# Skip slow AVIF tests
cargo test -- --skip avif

# Parallel execution (default)
cargo test -- --test-threads=8
```

### Missing fixtures
```bash
cd dpf && cargo run --example gen_fixtures
```

---

## 📊 Test Status

```
Rust Unit:        280+ ✅
Rust Integration:  20+ ✅
Go Tests:          16+ ✅
────────────────────────────
Total:           316+ ✅
```

Last updated: March 2026
