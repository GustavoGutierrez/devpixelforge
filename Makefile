.PHONY: build build-rust build-go clean test bench

# ─── Configuración ─────────────────────────────────────────────
RUST_DIR     := rust-imgproc
GO_DIR       := go-bridge
BIN_NAME     := devforge-imgproc
RUST_TARGET  := release

# Detectar OS para cross-compile
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
    RUST_TRIPLE := x86_64-unknown-linux-musl
endif
ifeq ($(UNAME_S),Darwin)
    RUST_TRIPLE := aarch64-apple-darwin
endif

# ─── Build ─────────────────────────────────────────────────────

## Compilar todo
build: build-rust build-go

## Compilar el motor de imágenes Rust (release optimizado)
build-rust:
	@echo "🦀 Building Rust image processor..."
	cd $(RUST_DIR) && cargo build --release
	@echo "✅ Binary: $(RUST_DIR)/target/release/$(BIN_NAME)"
	@ls -lh $(RUST_DIR)/target/release/$(BIN_NAME) 2>/dev/null || true

## Compilar binario estático con musl (Linux)
build-rust-static:
	@echo "🦀 Building static Rust binary (musl)..."
	cd $(RUST_DIR) && cargo build --release --target x86_64-unknown-linux-musl
	@echo "✅ Static binary ready"

## Compilar el ejemplo Go
build-go: build-rust
	@echo "🐹 Building Go example..."
	cd $(GO_DIR)/example && go build -o ../../$(BIN_NAME)-example .
	@ln -sf $(RUST_DIR)/target/release/$(BIN_NAME) $(BIN_NAME)
	@echo "✅ Go example built (symlink ./$(BIN_NAME) → $(RUST_DIR)/target/release/$(BIN_NAME))"

# ─── Testing ───────────────────────────────────────────────────

## Verificar que el binario Rust funciona
test: build-rust
	@echo "🧪 Testing capabilities..."
	./$(RUST_DIR)/target/release/$(BIN_NAME) caps

## Benchmark con una imagen de prueba
bench: build-rust
	@echo "⏱️  Benchmark: resize 5 sizes..."
	@echo '{"operation":"resize","input":"test.png","output_dir":"/tmp/bench","widths":[320,640,1024,1440,1920]}' | \
		time ./$(RUST_DIR)/target/release/$(BIN_NAME)

# ─── Clean ─────────────────────────────────────────────────────

clean:
	cd $(RUST_DIR) && cargo clean
	rm -f $(BIN_NAME)-example
	rm -rf /tmp/bench

# ─── Info ──────────────────────────────────────────────────────

## Mostrar tamaño del binario
size: build-rust
	@echo "📦 Binary size:"
	@ls -lh $(RUST_DIR)/target/release/$(BIN_NAME)
	@echo "📦 Stripped:"
	@strip $(RUST_DIR)/target/release/$(BIN_NAME) 2>/dev/null; \
		ls -lh $(RUST_DIR)/target/release/$(BIN_NAME)
