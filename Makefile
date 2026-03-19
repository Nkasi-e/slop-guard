SHELL := /bin/bash

ROOT_DIR := $(abspath .)
EXT_DIR := $(ROOT_DIR)/extension
ENGINE_DIR := $(ROOT_DIR)/engine
VSCE := npx @vscode/vsce@2.26.0
WASI_SYSROOT ?= /opt/homebrew/opt/wasi-libc/share/wasi-sysroot
CC_WASM ?= clang --target=wasm32-wasip1 --sysroot=$(WASI_SYSROOT)

.PHONY: help deps compile engine wasm vsce package repack bump-patch bump-minor bump-major release-patch release-minor release-major install-vsix publish-info clean-vsix

help:
	@echo "SlopGuard deployment helpers"
	@echo ""
	@echo "Targets:"
	@echo "  make deps         Install extension dependencies"
	@echo "  make compile      Compile extension TypeScript"
	@echo "  make engine       Build native Rust engine (release)"
	@echo "  make wasm         Build WASM fallback and copy to runtime/wasm/"
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

wasm:
	cd "$(ENGINE_DIR)" && \
	WASI_SYSROOT="$(WASI_SYSROOT)" \
	CC_wasm32_wasip1="$(CC_WASM)" \
	cargo build --release --target wasm32-wasip1 --no-default-features && \
	mkdir -p "$(EXT_DIR)/runtime/wasm" && \
	cp target/wasm32-wasip1/release/slopguard_engine.wasm "$(EXT_DIR)/runtime/wasm/slopguard_engine.wasm"

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
