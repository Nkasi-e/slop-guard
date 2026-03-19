Bundled engine binaries for marketplace releases.

## Native binaries

Place platform-specific engine executables in the following folders:

- `darwin-arm64/slopguard-engine`
- `darwin-x64/slopguard-engine`
- `linux-x64/slopguard-engine`
- `win32-x64/slopguard-engine.exe`
- `win32-arm64/slopguard-engine.exe`

Current status:

- `darwin-arm64`: built and copied
- `darwin-x64`: built and copied
- `linux-x64`: requires `x86_64-linux-gnu-gcc` (cross-compile on macOS) or native Linux build
- `win32-x64`: requires `x86_64-w64-mingw32-gcc` (cross-compile on macOS) or native Windows build
- `win32-arm64`: requires native Windows ARM build

## WASM fallback

`wasm/slopguard_engine.wasm` — **built and ready** (218 KB).

Built with `--no-default-features` (tree-sitter AST analysis excluded; pattern, complexity,
and idiomatic analyzers included). This is the fallback for any platform without a native binary.

### Rebuild WASM

```bash
cd engine
WASI_SYSROOT=/opt/homebrew/opt/wasi-libc/share/wasi-sysroot \
  CC_wasm32_wasip1="clang --target=wasm32-wasip1 --sysroot=/opt/homebrew/opt/wasi-libc/share/wasi-sysroot" \
  cargo build --release --target wasm32-wasip1 --no-default-features
cp target/wasm32-wasip1/release/slopguard_engine.wasm ../extension/runtime/wasm/
```

Before packaging (`vsce package`), ensure all native binaries are in place.
The WASM fallback covers any gap automatically.
