#![allow(unused_variables)]

use crate::{wasi_stderr, wasi_stdout, WasiCtx};

#[async_trait::async_trait]
impl wasi_stdout::WasiStdout for WasiCtx {
    async fn log(
        &mut self,
        _level: wasi_stdout::Level,
        _context: String,
        message: String,
    ) -> anyhow::Result<()> {
        print!("{}", message);
        Ok(())
    }
}

#[async_trait::async_trait]
impl wasi_stderr::WasiStderr for WasiCtx {
    async fn log(
        &mut self,
        _level: wasi_stderr::Level,
        _context: String,
        message: String,
    ) -> anyhow::Result<()> {
        eprint!("{}", message);
        Ok(())
    }
}
