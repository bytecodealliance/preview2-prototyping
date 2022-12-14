#![allow(unused_variables)]

use crate::{wasi_exit, WasiCtx};

#[async_trait::async_trait]
impl wasi_exit::WasiExit for WasiCtx {
    async fn exit(&mut self, status: Result<(), ()>) -> anyhow::Result<()> {
        let status = match status {
            Ok(()) => 0,
            Err(()) => 1,
        };
        std::process::exit(status)
    }
}
