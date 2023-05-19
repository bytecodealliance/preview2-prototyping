# preview2-prototyping

## This repository will soon be retired!

This repository was home to the prototype implementation of Wasmtime's WASI
Preview 2 support for 7 months. We have now landed almost all of this
repository upstream:

* [Wasmtime issue
  #6370](https://github.com/bytecodealliance/wasmtime/issues/6370) describes
  the transition plan, and tracks its progress.

* The component adapter, which used to live at the root of this repo and now
  lives at `crates/wasi-preview1-component-adapter`, has a new home at
  [`wasmtime/crates/wasi-preview1-component-adapter`](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-preview1-component-adapter).

* Published binaries of the component adapter can now be found in the
  [Wasmtime `dev` tag
  assets](https://github.com/bytecodealliance/wasmtime/releases/tag/dev)
  under [`wasi_preview1_component_adapter.command.wasm`](https://github.com/bytecodealliance/wasmtime/releases/download/dev/wasi_preview1_component_adapter.command.wasm)
  and [`wasi_preview1_component_adapter.reactor.wasm`](https://github.com/bytecodealliance/wasmtime/releases/download/dev/wasi_preview1_component_adapter.reactor.wasm).

* If you are invoking `wasm-tools component new` with the component adapter,
  you now need to specify the name to adapt is `wasi_snapshot_preview1`, e.g.
  `wasm-tools component new --adapt wasi_snapshot_preview1=./wasi_preview1_component_adapter.command.wasm`

* The host implementation of WASI Preview 2 was found in this repository's
  `wasi-common` crate. The new home is in the
  [`wasmtime-wasi` crate under the `preview2` module](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi/src/preview2).

* If you were vendoring in the `wasi-common` from this repository, change your
  source code's `use wasi_common::{WasiCtxBuilder, WasiView, wasi::Command}`
  to `use wasmtime_wasi::preview2::{WasiCtxBuilder, WasiView, wasi::Command}`
  to switch to the new implementation.

* If you are looking for the tests found under this repository's `host/tests`,
  those now live at
  [`wasmtime/crates/test-programs/tests`](https://github.com/bytecodealliance/wasmtime/tree/main/crates/test-programs/tests).

* If you are looking for the contents of this repository's `test-programs`,
  those now live under
  [`wasmtime/crates/test-programs`](https://github.com/bytecodealliance/wasmtime/tree/main/crates/test-programs).


## Still active in this repository

The `wasi-sockets` implementation in this repository is still a work in
progress. It will land in Wasmtime in the future, but still lives here
for the time being.
