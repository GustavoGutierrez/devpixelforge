# Tasks: Rename dev-forge-imgproc to devpixelforge

## Phase 1: Foundation (Package/Module Identity)
**Prerequisite**: None. These changes enable all subsequent tasks.

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| 1.1 | Update `package.name` to "dpf" and `package.description` to reference devpixelforge | `rust-imgproc/Cargo.toml` | Low | None |
| 1.2 | Update `[[bin]].name` to "dpf" | `rust-imgproc/Cargo.toml` | Low | None |
| 1.3 | Update module path to `github.com/GustavoGutierrez/devpixelforge-bridge` | `go-bridge/go.mod` | Low | None |

## Phase 2: Core Implementation (Source Code)
**Prerequisite**: Phase 1 complete (go.mod updated for import resolution).

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| 2.1 | Update CLI name to "dpf" and about text to "devpixelforge" | `rust-imgproc/src/main.rs` | Low | 1.1, 1.2 |
| 2.2 | Update comments referencing devforge-imgproc binary | `go-bridge/imgproc.go` | Low | None |
| 2.3 | Update import path and binary path references | `go-bridge/example/main.go` | Low | 1.3 |
| 2.4 | Update `BIN_NAME` variable to "dpf" | `Makefile` | Low | None |
| 2.5 | Update binary and example names in ignore patterns | `.gitignore` | Low | None |

## Phase 3: Integration (Dependency Lock & Imports)
**Prerequisite**: Phase 1 and 2 complete.

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| 3.1 | Regenerate Cargo.lock after Cargo.toml name change | `rust-imgproc/Cargo.lock` | Low | 1.1, 1.2 |
| 3.2 | Run `go mod tidy` to update go.sum and verify module | `go-bridge/go.sum` (auto) | Low | 1.3, 2.3 |

## Phase 4: Documentation (User-Facing Content)
**Prerequisite**: Phase 1 and 2 complete (names finalized).

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| 4.1 | Update title, badges, installation, and usage examples | `README.md` | Medium | 2.1, 2.4 |
| 4.2 | Update binary references and client paths | `INTEGRATION.md` | Low | 2.1 |
| 4.3 | Update project title in skill registry | `.atl/skill-registry.md` | Low | None |

## Phase 5: Verification (Testing & Validation)
**Prerequisite**: All previous phases complete.

| # | Task | Action | Complexity | Dependencies |
|---|------|--------|------------|--------------|
| 5.1 | Verify Rust build produces dpf binary | `cargo build --release` in rust-imgproc/ | Low | 3.1 |
| 5.2 | Verify binary version shows devpixelforge | `./target/release/dpf --version` | Low | 5.1 |
| 5.3 | Verify Go build succeeds | `go build ./...` in go-bridge/ | Low | 3.2 |
| 5.4 | Verify no old name references remain | `grep -r "devforge-imgproc\|dev-forge-imgproc" --include="*.rs" --include="*.go" --include="*.md" --include="*.toml" --include="Makefile" .` | Low | All |

---

## Implementation Notes

### Complexity Legend
- **Low**: Simple string replacement, 1-2 line changes
- **Medium**: Coordinated changes across multiple sections
- **High**: Requires design decisions or extensive refactoring

### Task Dependencies Graph
```
1.1, 1.2 ─┬─→ 2.1 ──┬─→ 3.1 ──→ 5.1 ──→ 5.2
          │         │
1.3 ──────┴─→ 2.3 ──┴─→ 3.2 ──→ 5.3
          │
          ├──→ 2.2 (independent)
          │
2.4, 2.5 ─┤
          │
4.1, 4.2, 4.3 ──→ 5.4 (grep validation)
```

### Rollback Plan
All changes can be reverted via: `git revert HEAD` (single coordinated commit).
