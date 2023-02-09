use crate::{
    wasi_http::{Request, Response, WasiHttp},
    WasiCtx
};
use reqwest::{Client, Method};

#[async_trait::async_trait]
impl WasiHttp for WasiCtx {
    async fn make_request(&mut self, req: Request) -> anyhow::Result<Response> {
        let client = Client::default();
        let mut builder = client.request(
            Method::from_bytes(req.method.as_bytes())?,
            req.uri,
        );
        for header in req.headers {
            builder = builder.header(header.0, header.1);
        }
        let res = builder.send().await?;
        let status = res.status().as_u16();
        let mut headers = vec![];
        for (name, value) in res.headers().iter() {
            headers.push((
                name.as_str().to_string(),
                value.to_str()?.to_string(),
            ));
        }
        let body = Some(res.bytes().await?.to_vec());
        Ok(Response {
            status,
            headers: Some(headers),
            body,
        })
    }
}
