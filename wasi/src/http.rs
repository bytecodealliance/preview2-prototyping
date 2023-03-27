use crate::snapshots::preview_2::{default_outgoing_http, streams, types as http_types};
use http::header::{HeaderName, HeaderValue};
use std::{ops::Deref, str::FromStr};

pub struct DefaultClient {
    options: Option<default_outgoing_http::RequestOptions>,
}

impl DefaultClient {
    pub fn new(options: Option<default_outgoing_http::RequestOptions>) -> Self {
        Self { options }
    }

    pub fn handle(&self, req: http::Request<Vec<u8>>) -> anyhow::Result<http::Response<Vec<u8>>> {
        let req = Request::from(req).to_owned();
        let res = default_outgoing_http::handle(req, self.options);
        let res: http::Response<Vec<u8>> = Response(res).into();
        http_types::drop_outgoing_request(req);
        Ok(res)
    }
}

pub struct Request(default_outgoing_http::OutgoingRequest);

impl Deref for Request {
    type Target = default_outgoing_http::OutgoingRequest;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<http::Request<T>> for Request {
    fn from(req: http::Request<T>) -> Self {
        let (parts, _) = req.into_parts();
        let path = parts.uri.path();
        let query = parts.uri.query();
        let method = Method::from(parts.method);
        let headers = Headers::from(&parts.headers);
        let scheme = match parts.uri.scheme_str().unwrap_or("") {
            "http" => Some(http_types::SchemeParam::Http),
            "https" => Some(http_types::SchemeParam::Https),
            _ => None,
        };
        Self(http_types::new_outgoing_request(
            method.to_owned(),
            path,
            query.unwrap_or(""),
            scheme,
            parts.uri.authority().map(|a| a.as_str()).unwrap(),
            headers.to_owned(),
        ))
    }
}

pub struct Method<'a>(http_types::MethodParam<'a>);

impl<'a> Deref for Method<'a> {
    type Target = http_types::MethodParam<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<http::Method> for Method<'a> {
    fn from(method: http::Method) -> Self {
        Self(match method {
             http::Method::GET => http_types::MethodParam::Get,
             http::Method::POST => http_types::MethodParam::Post,
             http::Method::PUT => http_types::MethodParam::Put,
             http::Method::DELETE => http_types::MethodParam::Delete,
             http::Method::PATCH => http_types::MethodParam::Patch,
             http::Method::CONNECT => http_types::MethodParam::Connect,
             http::Method::TRACE => http_types::MethodParam::Trace,
             http::Method::HEAD => http_types::MethodParam::Head,
             http::Method::OPTIONS => http_types::MethodParam::Options,
             _ => panic!("failed due to unsupported method, currently supported methods are: GET, POST, PUT, DELETE, PATCH, CONNECT, TRACE, HEAD, and OPTIONS"),
         })
    }
}

pub struct Response(http_types::IncomingResponse);

impl Deref for Response {
    type Target = http_types::IncomingResponse;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Response> for http::Response<Vec<u8>> {
    fn from(val: Response) -> Self {
        let res_pointer = val.to_owned();
        // TODO: Run drop when implemented
        // poll::drop_pollable(res_pointer);
        let status = http_types::incoming_response_status(res_pointer);
        let header_handle = http_types::incoming_response_headers(res_pointer);
        let headers = http_types::fields_entries(header_handle);
        let stream = http_types::incoming_response_consume(res_pointer).unwrap();
        let len = 64 * 1024;
        let mut body: Vec<u8> = vec![];
        loop {
            let (b, finished) = streams::read(stream, len).unwrap();
            body.extend(b);
            if finished {
                break;
            }
        }
        let mut res = http::Response::builder().status(status).body(body).unwrap();
        let headers_map = res.headers_mut();
        for (name, value) in headers {
            headers_map.insert(
                HeaderName::from_str(name.as_ref()).unwrap(),
                HeaderValue::from_str(value.as_str()).unwrap(),
            );
        }
        streams::drop_input_stream(stream);
        http_types::drop_incoming_response(res_pointer);
        res
    }
}

pub struct Headers(http_types::Fields);

impl Deref for Headers {
    type Target = http_types::Fields;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a http::HeaderMap> for Headers {
    fn from(headers: &'a http::HeaderMap) -> Self {
        Self(http_types::new_fields(
            &headers
                .iter()
                .map(|(name, value)| (name.as_str(), value.to_str().unwrap()))
                .collect::<Vec<(&'a str, &'a str)>>(),
        ))
    }
}
