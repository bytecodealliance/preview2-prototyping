use crate::{
    wasi_http::{HttpError, Method, Request, Response, WasiHttp},
    HostResult, WasiCtx
};

#[async_trait::async_trait]
impl WasiHttp for WasiCtx {
    async fn send(&mut self, req: Request) -> HostResult<Response, HttpError> {
        let client = reqwest::Client::default();
        let mut builder = client.request(
            req.method.into(),
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

impl From<Method> for reqwest::Method {
    fn from(method: Method) -> Self {
        match method {
           Method::Get => reqwest::Method::GET,
           Method::Post => reqwest::Method::POST,
           Method::Put => reqwest::Method::PUT,
           Method::Delete => reqwest::Method::DELETE,
           Method::Patch => reqwest::Method::PATCH,
           Method::Head => reqwest::Method::HEAD,
           Method::Options => reqwest::Method::OPTIONS,
            _ => panic!("failed due to unsupported method, currently supported methods are: GET, POST, PUT, DELETE, PATCH, HEAD, and OPTIONS"),
        }
    }
}
