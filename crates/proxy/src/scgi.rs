//! SCGI protocol client implementation for upstream proxying.

use anyhow::Result;
use bytes::{BufMut, Bytes, BytesMut};
use http_body_util::BodyExt;
use hyper::Request;
use std::collections::HashMap;

pub struct ScgiClient;

impl ScgiClient {
    /// Generates the exact byte sequence needed to initiate an SCGI request.
    /// Format: `[length]:[headers],` followed by the request body
    pub async fn encode_request<B>(req: Request<B>) -> Result<Bytes>
    where
        B: http_body::Body<Data = Bytes> + Unpin,
        B::Error: Into<anyhow::Error>,
    {
        let mut headers = HashMap::new();

        // 1. Mandatory SCGI variable
        headers.insert("SCGI".to_string(), "1".to_string());

        // 2. Add standard CGI/HTTP metadata
        let method = req.method().as_str().to_string();
        headers.insert("REQUEST_METHOD".to_string(), method);
        headers.insert("REQUEST_URI".to_string(), req.uri().path().to_string());

        if let Some(query) = req.uri().query() {
            headers.insert("QUERY_STRING".to_string(), query.to_string());
        }

        // Add HTTP headers
        for (name, value) in req.headers() {
            let key = format!("HTTP_{}", name.as_str().to_uppercase().replace("-", "_"));
            if let Ok(v) = value.to_str() {
                headers.insert(key, v.to_string());
            }
        }

        // Ensure CONTENT_LENGTH exists (SCGI requires it)
        if !headers.contains_key("HTTP_CONTENT_LENGTH") && !headers.contains_key("CONTENT_LENGTH") {
            headers.insert("CONTENT_LENGTH".to_string(), "0".to_string());
        }

        // 3. Serialize headers into a netstring payload
        // Format: `NAME\0VALUE\0`
        let mut header_payload = BytesMut::new();

        // SCGI must appear first according to spec
        header_payload.put_slice(b"SCGI\0");
        header_payload.put_slice(b"1\0");
        headers.remove("SCGI");

        // CONTENT_LENGTH must appear second according to spec
        let content_length = headers
            .remove("CONTENT_LENGTH")
            .or_else(|| headers.remove("HTTP_CONTENT_LENGTH"))
            .unwrap_or_else(|| "0".to_string());
        header_payload.put_slice(b"CONTENT_LENGTH\0");
        header_payload.put_slice(content_length.as_bytes());
        header_payload.put_u8(0);

        for (k, v) in headers {
            header_payload.put_slice(k.as_bytes());
            header_payload.put_u8(0);
            header_payload.put_slice(v.as_bytes());
            header_payload.put_u8(0);
        }

        // 4. Construct the final SCGI netstring
        let mut buf = BytesMut::new();
        let length_str = format!("{}:", header_payload.len());

        buf.put_slice(length_str.as_bytes());
        buf.put(header_payload.freeze());
        buf.put_u8(b',');

        // 5. Append Body
        let mut body = req.into_body();
        while let Some(chunk_res) = body.frame().await {
            let frame = chunk_res.map_err(|e| e.into())?;
            if let Some(chunk) = frame.data_ref() {
                buf.put(chunk.clone());
            }
        }

        Ok(buf.freeze())
    }
}
