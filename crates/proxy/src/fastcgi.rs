//! FastCGI (FCGI) protocol client implementation for upstream proxying.

use anyhow::Result;
use bytes::{BufMut, Bytes, BytesMut};
use http_body_util::BodyExt;
use hyper::Request;
use std::collections::HashMap;

const FCGI_VERSION_1: u8 = 1;
const FCGI_BEGIN_REQUEST: u8 = 1;
const FCGI_ABORT_REQUEST: u8 = 2;
const FCGI_END_REQUEST: u8 = 3;
const FCGI_PARAMS: u8 = 4;
const FCGI_STDIN: u8 = 5;
const FCGI_STDOUT: u8 = 6;
const FCGI_STDERR: u8 = 7;
const FCGI_DATA: u8 = 8;
const FCGI_GET_VALUES: u8 = 9;
const FCGI_GET_VALUES_RESULT: u8 = 10;
const FCGI_UNKNOWN_TYPE: u8 = 11;

const FCGI_RESPONDER: u16 = 1;

/// Represents a FastCGI record header
#[derive(Debug)]
struct FcgiHeader {
    version: u8,
    type_: u8,
    request_id: u16,
    content_length: u16,
    padding_length: u8,
    reserved: u8,
}

impl FcgiHeader {
    fn new(type_: u8, request_id: u16, content_length: u16, padding_length: u8) -> Self {
        Self {
            version: FCGI_VERSION_1,
            type_,
            request_id,
            content_length,
            padding_length,
            reserved: 0,
        }
    }

    fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u8(self.version);
        buf.put_u8(self.type_);
        buf.put_u16(self.request_id);
        buf.put_u16(self.content_length);
        buf.put_u8(self.padding_length);
        buf.put_u8(self.reserved);
    }
}

pub struct FastCgiClient;

impl FastCgiClient {
    /// Generates exactly the byte sequence needed to initiate a complete FastCGI Request
    pub async fn encode_request<B>(
        req: Request<B>,
        request_id: u16,
        script_filename: &str,
    ) -> Result<bytes::Bytes>
    where
        B: http_body::Body<Data = bytes::Bytes> + Unpin,
        B::Error: Into<anyhow::Error>,
    {
        let mut buf = BytesMut::new();

        // 1. FCGI_BEGIN_REQUEST
        let begin_req_body = [
            (FCGI_RESPONDER >> 8) as u8,   // roleB1
            (FCGI_RESPONDER & 0xFF) as u8, // roleB0
            0,                             // flags (0 = connection close)
            0,
            0,
            0,
            0,
            0, // reserved
        ];

        FcgiHeader::new(FCGI_BEGIN_REQUEST, request_id, 8, 0).write_to(&mut buf);
        buf.put_slice(&begin_req_body);

        // 2. FCGI_PARAMS
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("SCRIPT_FILENAME".to_string(), script_filename.to_string());
        params.insert(
            "REQUEST_METHOD".to_string(),
            req.method().as_str().to_string(),
        );
        params.insert("REQUEST_URI".to_string(), req.uri().path().to_string());
        if let Some(query) = req.uri().query() {
            params.insert("QUERY_STRING".to_string(), query.to_string());
        }

        // Add HTTP headers
        for (name, value) in req.headers() {
            let key = format!("HTTP_{}", name.as_str().to_uppercase().replace("-", "_"));
            if let Ok(v) = value.to_str() {
                params.insert(key, v.to_string());
            }
        }

        let mut params_buf = BytesMut::new();
        for (k, v) in params {
            Self::encode_nv_pair(&k, &v, &mut params_buf);
        }

        // Write the Params payload in chunks of up to 65535 bytes
        let mut params_bytes = params_buf.freeze();
        while !params_bytes.is_empty() {
            let chunk_len = std::cmp::min(params_bytes.len(), 65535);
            let chunk = params_bytes.split_to(chunk_len);
            let pad_len = (8 - (chunk_len % 8)) % 8;

            FcgiHeader::new(FCGI_PARAMS, request_id, chunk_len as u16, pad_len as u8)
                .write_to(&mut buf);
            buf.put(chunk);
            buf.put_bytes(0, pad_len);
        }

        // Terminating empty FCGI_PARAMS record
        FcgiHeader::new(FCGI_PARAMS, request_id, 0, 0).write_to(&mut buf);

        // 3. FCGI_STDIN
        // We buffer the whole body. A full streaming integration requires a chunked encoder layer.
        let mut body = req.into_body();
        while let Some(chunk_res) = body.frame().await {
            let frame = chunk_res.map_err(|e| e.into())?;
            if let Some(chunk) = frame.data_ref() {
                let mut chunk_bytes = chunk.clone();
                while !chunk_bytes.is_empty() {
                    let write_len = std::cmp::min(chunk_bytes.len(), 65535);
                    let to_write = chunk_bytes.split_to(write_len);
                    let pad_len = (8 - (write_len % 8)) % 8;

                    FcgiHeader::new(FCGI_STDIN, request_id, write_len as u16, pad_len as u8)
                        .write_to(&mut buf);
                    buf.put(to_write);
                    buf.put_bytes(0, pad_len);
                }
            }
        }

        // Terminating empty FCGI_STDIN record
        FcgiHeader::new(FCGI_STDIN, request_id, 0, 0).write_to(&mut buf);

        Ok(buf.freeze())
    }

    fn encode_nv_pair(name: &str, value: &str, buf: &mut BytesMut) {
        let name_len = name.len();
        let val_len = value.len();

        if name_len < 128 {
            buf.put_u8(name_len as u8);
        } else {
            buf.put_u32(name_len as u32 | 0x80000000);
        }

        if val_len < 128 {
            buf.put_u8(val_len as u8);
        } else {
            buf.put_u32(val_len as u32 | 0x80000000);
        }

        buf.put_slice(name.as_bytes());
        buf.put_slice(value.as_bytes());
    }
}
