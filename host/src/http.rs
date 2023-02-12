use crate::{
    wasi_http::{HttpError, Request, Response, WasiHttp},
    HostResult, WasiCtx
};
use reqwest::{Client, Method};

#[async_trait::async_trait]
impl WasiHttp for WasiCtx {
    async fn send(&mut self, req: Request) -> HostResult<Response, HttpError> {
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
        Ok(Ok(Response {
            status,
            headers: Some(headers),
            body,
        }))
    }
}

impl From<reqwest::Error> for HttpError {
    fn from(e: reqwest::Error) -> Self {
        Self::UnexpectedError(e.to_string())
    }
}
