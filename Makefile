SHELL := /bin/bash

ROOT_DIR := $(abspath .)
EXT_DIR := $(ROOT_DIR)/extension
ENGINE_DIR := $(ROOT_DIR)/engine
VSCE := npx @vscode/vsce@2.26.0
WASI_SYSROOT ?= /opt/homebrew/opt/wasi-libc/share/wasi-sysroot
CC_WASM ?= clang --target=wasm32-wasip1 --sysroot=$(WASI_SYSROOT)

.PHONY: help deps compile engine wasm package repack install-vsix publish-info clean-vsix

help:
	@echo "SlopGuard deployment helpers"
	@echo ""
	@echo "Targets:"
	@echo "  make deps         Install extension dependencies"
	@echo "  make compile      Compile extension TypeScript"
	@echo "  make engine       Build native Rust engine (release)"
	@echo "  make wasm         Build WASM fallback and copy to runtime/wasm/"
	@echo "  make package      Build + package VSIX"
	@echo "  make repack       Alias for package"
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

package: compile
	cd "$(EXT_DIR)" && $(VSCE) package

repack: package

install-vsix: package
	code --install-extension "$(EXT_DIR)/slopguard-0.0.1.vsix"

publish-info:
	@echo "Manual publish flow:"
	@echo "  1) make package"
	@echo "  2) Upload $(EXT_DIR)/slopguard-0.0.1.vsix in VS Marketplace Publisher Portal"

clean-vsix:
	rm -f "$(EXT_DIR)"/*.vsix
