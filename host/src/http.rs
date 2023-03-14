use crate::{
    wasi,
    wasi::http_types::{IncomingRequest as Request, ResponseOutparam as Response},
    WasiCtx,
};

#[async_trait::async_trait]
impl wasi::http::Host for WasiCtx {
    async fn handle<'a>(&'a mut self, _req: Request, _resp: Response) -> anyhow::Result<()> {
        todo!()
    }
}
