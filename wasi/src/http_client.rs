use anyhow::{anyhow, Context};
use bytes::{BufMut, Bytes, BytesMut};
use core::ops::Deref;
use http::header::{HeaderName, HeaderValue};

use crate::snapshots::preview_2::wasi::http::{outgoing_handler, types as http_types};
use crate::snapshots::preview_2::wasi::io::streams;
use crate::snapshots::preview_2::wasi::poll::poll;

struct DropPollable {
    pollable: poll::Pollable,
}

impl Drop for DropPollable {
    fn drop(&mut self) {
        poll::drop_pollable(self.pollable);
    }
}

pub struct DefaultClient {
    options: Option<outgoing_handler::RequestOptions>,
}

impl DefaultClient {
    pub fn new(options: Option<outgoing_handler::RequestOptions>) -> Self {
        Self { options }
    }

    pub fn handle(&self, req: http::Request<Bytes>) -> anyhow::Result<http::Response<Bytes>> {
        let req = Request::try_from(req).context("converting http request")?;

        let res = outgoing_handler::handle(req.id, self.options);

        let response =
            http::Response::try_from(Response(res)).context("converting http response")?;

        streams::drop_output_stream(req.body);
        http_types::drop_outgoing_request(req.id);

        Ok(response)
    }
}

#[derive(Default, Debug, Clone)]
pub struct Request {
    id: outgoing_handler::OutgoingRequest,
    body: http_types::OutgoingStream,
}

impl Request {
    pub fn new(id: outgoing_handler::OutgoingRequest, body: http_types::OutgoingStream) -> Self {
        Self { id, body }
    }
}

impl TryFrom<http::Request<Bytes>> for Request {
    type Error = anyhow::Error;

    fn try_from(value: http::Request<Bytes>) -> Result<Self, Self::Error> {
        let (parts, body) = value.into_parts();
        let method = Method::try_from(parts.method).context("converting request method")?;
        let path_with_query = parts.uri.path_and_query();
        let headers = Headers::from(&parts.headers);
        let scheme = match parts.uri.scheme_str().unwrap_or("") {
            "http" => Some(&http_types::Scheme::Http),
            "https" => Some(&http_types::Scheme::Https),
            _ => None,
        };
        let request = http_types::new_outgoing_request(
            &method,
            path_with_query.map(|q| q.as_str()),
            scheme,
            parts.uri.authority().map(|a| a.as_str()),
            headers.to_owned(),
        );

        let request_body = http_types::outgoing_request_write(request)
            .map_err(|_| anyhow!("outgoing request write failed"))?;

        if body.is_empty() {
            let sub = DropPollable {
                pollable: streams::subscribe_to_output_stream(request_body),
            };
            let mut buf = body.as_ref();
            while !buf.is_empty() {
                poll::poll_oneoff(&[sub.pollable]);

                let permit = match streams::check_write(request_body) {
                    Ok(n) => usize::try_from(n)?,
                    Err(_) => anyhow::bail!("output stream error"),
                };

                let len = buf.len().min(permit);
                let (chunk, rest) = buf.split_at(len);
                buf = rest;

                if streams::write(request_body, chunk).is_err() {
                    anyhow::bail!("output stream error")
                }
            }

            if streams::flush(request_body).is_err() {
                anyhow::bail!("output stream error")
            }

            poll::poll_oneoff(&[sub.pollable]);

            if streams::check_write(request_body).is_err() {
                anyhow::bail!("output stream error")
            }
        }

        Ok(Request::new(request, request_body))
    }
}

pub struct Method(http_types::Method);

impl Deref for Method {
    type Target = http_types::Method;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<http::Method> for Method {
    type Error = anyhow::Error;

    fn try_from(method: http::Method) -> Result<Self, Self::Error> {
        Ok(Self(match method {
            http::Method::GET => http_types::Method::Get,
            http::Method::POST => http_types::Method::Post,
            http::Method::PUT => http_types::Method::Put,
            http::Method::DELETE => http_types::Method::Delete,
            http::Method::PATCH => http_types::Method::Patch,
            http::Method::CONNECT => http_types::Method::Connect,
            http::Method::TRACE => http_types::Method::Trace,
            http::Method::HEAD => http_types::Method::Head,
            http::Method::OPTIONS => http_types::Method::Options,
            _ => return Err(anyhow!("failed due to unsupported method, currently supported methods are: GET, POST, PUT, DELETE, PATCH, CONNECT, TRACE, HEAD, and OPTIONS")),
        }))
    }
}

pub struct Response(http_types::IncomingResponse);

impl Deref for Response {
    type Target = http_types::IncomingResponse;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for http::Response<Bytes> {
    type Error = anyhow::Error;

    fn try_from(value: Response) -> Result<Self, Self::Error> {
        let future_response = value.to_owned();

        let incoming_response = match http_types::future_incoming_response_get(future_response) {
            Some(result) => result,
            None => {
                let pollable = http_types::listen_to_future_incoming_response(future_response);
                let _ = poll::poll_oneoff(&[pollable]);
                http_types::future_incoming_response_get(future_response)
                    .expect("incoming response available")
            }
        }
        .map_err(|e| anyhow!("incoming response error: {e:?}"))?;

        http_types::drop_future_incoming_response(future_response);

        let status = http_types::incoming_response_status(incoming_response);

        let body_stream = http_types::incoming_response_consume(incoming_response)
            .map_err(|_| anyhow!("consuming incoming response"))?;

        let mut body = BytesMut::new();
        {
            let sub = DropPollable {
                pollable: streams::subscribe_to_input_stream(body_stream),
            };
            poll::poll_oneoff(&[sub.pollable]);
            let mut eof = streams::StreamStatus::Open;
            while eof != streams::StreamStatus::Ended {
                let (body_chunk, stream_status) = streams::read(body_stream, u64::MAX)
                    .map_err(|e| anyhow!("reading response body: {e:?}"))?;
                eof = if body_chunk.is_empty() {
                    streams::StreamStatus::Ended
                } else {
                    stream_status
                };
                body.put(body_chunk.as_slice());
            }
        }

        let mut res = http::Response::builder()
            .status(status)
            .body(body.freeze())
            .map_err(|_| anyhow!("building http response"))?;

        streams::drop_input_stream(body_stream);

        let headers_handle = http_types::incoming_response_headers(incoming_response);
        if headers_handle > 0 {
            let headers_map = res.headers_mut();
            for (name, value) in http_types::fields_entries(headers_handle) {
                headers_map.insert(
                    HeaderName::from_bytes(name.as_bytes())
                        .map_err(|_| anyhow!("converting response header name"))?,
                    HeaderValue::from_bytes(value.as_slice())
                        .map_err(|_| anyhow!("converting response header value"))?,
                );
            }
        }
        http_types::drop_fields(headers_handle);

        http_types::drop_incoming_response(incoming_response);

        Ok(res)
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
            headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_string()))
                .collect::<Vec<_>>()
                .as_slice(),
        ))
    }
}
