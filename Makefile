SHELL := /bin/bash

ROOT_DIR := $(abspath .)
EXT_DIR := $(ROOT_DIR)/extension
ENGINE_DIR := $(ROOT_DIR)/engine
VSCE := npx @vscode/vsce@2.26.0
WASI_SYSROOT ?= /opt/homebrew/opt/wasi-libc/share/wasi-sysroot
CC_WASM ?= clang --target=wasm32-wasip1 --sysroot=$(WASI_SYSROOT)

.PHONY: help deps compile engine engine-all engine-all-check wasm local-test local-engine-install vsce package repack bump-patch bump-minor bump-major release-patch release-minor release-major install-vsix publish-info clean-vsix

UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

help:
	@echo "SlopGuard deployment helpers"
	@echo ""
	@echo "Targets:"
	@echo "  make deps         Install extension dependencies"
	@echo "  make compile      Compile extension TypeScript"
	@echo "  make engine       Build native Rust engine (release)"
	@echo "  make engine-all   Build + copy all supported engine targets"
	@echo "  make engine-all-check Verify expected runtime artifacts exist"
	@echo "  make local-test   Local dev: deps + tests + host engine → runtime/ + wasm + compile (then F5 in extension/)"
	@echo "  make wasm         Build WASM fallback and copy to runtime/wasm/"
	@echo "  (CLI) cd engine && cargo build --release && ./target/release/slopguard-engine scan ."
	@echo "  make vsce         Run raw vsce package command"
	@echo "  make package      Build + package VSIX"
	@echo "  make repack       Alias for package"
	@echo "  make bump-patch   Bump extension patch version (no git tag)"
	@echo "  make bump-minor   Bump extension minor version (no git tag)"
	@echo "  make bump-major   Bump extension major version (no git tag)"
	@echo "  make release-patch Bump patch + package VSIX (one command)"
	@echo "  make release-minor Bump minor + package VSIX (one command)"
	@echo "  make release-major Bump major + package VSIX (one command)"
	@echo "  make install-vsix Install packaged VSIX locally (requires 'code' CLI)"
	@echo "  make publish-info Show manual publish steps"
	@echo "  make clean-vsix   Remove generated VSIX files"

deps:
	cd "$(EXT_DIR)" && npm install

compile:
	cd "$(EXT_DIR)" && npm run compile

engine:
	cd "$(ENGINE_DIR)" && cargo build --release

engine-all:
	cd "$(ENGINE_DIR)" && \
	CARGO_TARGET_DIR="$(ENGINE_DIR)/target" cargo build --release --target aarch64-apple-darwin && \
	CARGO_TARGET_DIR="$(ENGINE_DIR)/target" cargo build --release --target x86_64-apple-darwin && \
	CARGO_TARGET_DIR="$(ENGINE_DIR)/target" cargo zigbuild --release --target x86_64-unknown-linux-gnu && \
	CARGO_TARGET_DIR="$(ENGINE_DIR)/target" cargo zigbuild --release --target x86_64-pc-windows-gnu && \
	WASI_SYSROOT="$(WASI_SYSROOT)" \
	CC_wasm32_wasip1="$(CC_WASM)" \
	CARGO_TARGET_DIR="$(ENGINE_DIR)/target" cargo build --release --target wasm32-wasip1 --no-default-features
	mkdir -p "$(EXT_DIR)/runtime/darwin-arm64" "$(EXT_DIR)/runtime/darwin-x64" "$(EXT_DIR)/runtime/linux-x64" "$(EXT_DIR)/runtime/win32-x64" "$(EXT_DIR)/runtime/wasm"
	cp "$(ENGINE_DIR)/target/aarch64-apple-darwin/release/slopguard-engine" "$(EXT_DIR)/runtime/darwin-arm64/slopguard-engine"
	cp "$(ENGINE_DIR)/target/x86_64-apple-darwin/release/slopguard-engine" "$(EXT_DIR)/runtime/darwin-x64/slopguard-engine"
	cp "$(ENGINE_DIR)/target/x86_64-unknown-linux-gnu/release/slopguard-engine" "$(EXT_DIR)/runtime/linux-x64/slopguard-engine"
	cp "$(ENGINE_DIR)/target/x86_64-pc-windows-gnu/release/slopguard-engine.exe" "$(EXT_DIR)/runtime/win32-x64/slopguard-engine.exe"
	cp "$(ENGINE_DIR)/target/wasm32-wasip1/release/slopguard_engine.wasm" "$(EXT_DIR)/runtime/wasm/slopguard_engine.wasm"

engine-all-check:
	@test -x "$(EXT_DIR)/runtime/darwin-arm64/slopguard-engine"
	@test -x "$(EXT_DIR)/runtime/darwin-x64/slopguard-engine"
	@test -x "$(EXT_DIR)/runtime/linux-x64/slopguard-engine"
	@test -x "$(EXT_DIR)/runtime/win32-x64/slopguard-engine.exe"
	@test -f "$(EXT_DIR)/runtime/wasm/slopguard_engine.wasm"
	@echo "All engine-all artifacts are present."

wasm:
	cd "$(ENGINE_DIR)" && \
	WASI_SYSROOT="$(WASI_SYSROOT)" \
	CC_wasm32_wasip1="$(CC_WASM)" \
	cargo build --release --target wasm32-wasip1 --no-default-features && \
	mkdir -p "$(EXT_DIR)/runtime/wasm" && \
	cp target/wasm32-wasip1/release/slopguard_engine.wasm "$(EXT_DIR)/runtime/wasm/slopguard_engine.wasm"

# Copy host release binary into the folder the extension loads first (platform key).
local-engine-install:
	cd "$(ENGINE_DIR)" && cargo build --release
ifeq ($(UNAME_S),Darwin)
ifeq ($(UNAME_M),arm64)
	mkdir -p "$(EXT_DIR)/runtime/darwin-arm64"
	cp "$(ENGINE_DIR)/target/release/slopguard-engine" "$(EXT_DIR)/runtime/darwin-arm64/slopguard-engine"
	chmod +x "$(EXT_DIR)/runtime/darwin-arm64/slopguard-engine"
	@echo "Installed engine → extension/runtime/darwin-arm64/slopguard-engine"
else
	mkdir -p "$(EXT_DIR)/runtime/darwin-x64"
	cp "$(ENGINE_DIR)/target/release/slopguard-engine" "$(EXT_DIR)/runtime/darwin-x64/slopguard-engine"
	chmod +x "$(EXT_DIR)/runtime/darwin-x64/slopguard-engine"
	@echo "Installed engine → extension/runtime/darwin-x64/slopguard-engine"
endif
else ifeq ($(UNAME_S),Linux)
ifeq ($(UNAME_M),x86_64)
	mkdir -p "$(EXT_DIR)/runtime/linux-x64"
	cp "$(ENGINE_DIR)/target/release/slopguard-engine" "$(EXT_DIR)/runtime/linux-x64/slopguard-engine"
	chmod +x "$(EXT_DIR)/runtime/linux-x64/slopguard-engine"
	@echo "Installed engine → extension/runtime/linux-x64/slopguard-engine"
else
	@echo "local-engine-install: unsupported Linux arch $(UNAME_M); copy target/release/slopguard-engine manually"
endif
else
	@echo "local-engine-install: non-Unix or unknown OS; use engine/target/release or set slopguard.enginePath in VS Code"
endif

# One-shot prep for Extension Development Host (F5): bundled native + wasm + TS out/
local-test: deps compile
	cd "$(ENGINE_DIR)" && cargo test -q
	$(MAKE) local-engine-install
	$(MAKE) wasm
	@echo ""
	@echo "Local test ready."
	@echo "  1) Open the extension folder in VS Code: code $(EXT_DIR)"
	@echo "  2) Run “Run Extension” (F5)."
	@echo "  Engine CLI: $(ENGINE_DIR)/target/release/slopguard-engine scan $(ROOT_DIR) --no-fail"

vsce:
	cd "$(EXT_DIR)" && $(VSCE) package

package: compile
	$(MAKE) vsce

repack: package

bump-patch:
	cd "$(EXT_DIR)" && npm version patch --no-git-tag-version

bump-minor:
	cd "$(EXT_DIR)" && npm version minor --no-git-tag-version

bump-major:
	cd "$(EXT_DIR)" && npm version major --no-git-tag-version

release-patch: bump-patch package

release-minor: bump-minor package

release-major: bump-major package

install-vsix: package
	code --install-extension "$$(cd "$(EXT_DIR)" && ls -t slopguard-*.vsix | head -1)"

publish-info:
	@echo "Manual publish flow:"
	@echo "  1) make bump-patch   (or bump-minor / bump-major)"
	@echo "  2) make package"
	@echo "  3) Upload newest VSIX from $(EXT_DIR) in Publisher Portal"

clean-vsix:
	rm -f "$(EXT_DIR)"/*.vsix
