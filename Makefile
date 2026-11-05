CARGO   ?= cargo
OUT     := target/dist
NIGHTLY := -Zbuild-std=core,alloc
# ErisMacInjectLib as cargo resolves it: the monorepo's libs/ checkout, or the git dependency.
INJECT   = $(shell $(CARGO) metadata --format-version 1 | perl -ne 'print $$1 and exit if m{"manifest_path":"([^"]*)/crates/inject/Cargo.toml"}')

# macOS 26 only; also makes the linker emit chained fixups, which is what the injector reads.
export MACOSX_DEPLOYMENT_TARGET := 26.0

.PHONY: all payload thread cli test clean

all: payload thread cli

# One arm64 build, mapped into every app by the injector (arm64e apps included).
payload:
	$(CARGO) build --release -p payload $(NIGHTLY) --target aarch64-apple-darwin
	mkdir -p $(OUT)
	cp target/aarch64-apple-darwin/release/libliveresize.dylib $(OUT)/

# eris-inject's thread helper. Must be arm64e: it signs the payload thread's start for arm64e apps.
thread:
	$(CARGO) build --release -p eris-inject-thread $(NIGHTLY) --target arm64e-apple-darwin \
		--manifest-path $(INJECT)/Cargo.toml --target-dir target
	mkdir -p $(OUT)
	cp target/arm64e-apple-darwin/release/eris-inject-thread $(OUT)/

cli:
	$(CARGO) build --release -p liveresize
	mkdir -p $(OUT)
	cp target/release/liveresize $(OUT)/

test:
	$(CARGO) test

clean:
	$(CARGO) clean

