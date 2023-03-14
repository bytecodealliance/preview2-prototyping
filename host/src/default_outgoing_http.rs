use crate::{
    wasi,
    wasi::http_types::{
        Error as HttpError, FutureIncomingResponse as Response, Method, OutgoingRequest as Request,
        RequestOptions,
    },
    WasiCtx,
};

#[async_trait::async_trait]
impl wasi::default_outgoing_http::Host for WasiCtx {
    async fn handle<'a>(
        &'a mut self,
        _req: Request,
        _options: Option<RequestOptions>,
    ) -> wasmtime::Result<Response> {
        todo!()
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
           Method::Connect => reqwest::Method::CONNECT,
           Method::Trace => reqwest::Method::TRACE,
           Method::Head => reqwest::Method::HEAD,
           Method::Options => reqwest::Method::OPTIONS,
           _ => panic!("failed due to unsupported method, currently supported methods are: GET, POST, PUT, DELETE, PATCH, CONNECT, TRACE, HEAD, and OPTIONS"),
        }
    }
}
