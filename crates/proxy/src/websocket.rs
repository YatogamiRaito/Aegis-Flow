use hyper::{Request, Response, StatusCode};
use hyper::body::Bytes;
use http_body_util::combinators::BoxBody;
use tracing::{error, info, debug};
use hyper_util::rt::TokioIo;

pub fn is_websocket_upgrade<B>(req: &Request<B>) -> bool {
    let headers = req.headers();
    
    let is_upgrade = headers.get(hyper::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_ascii_lowercase().contains("upgrade"))
        .unwrap_or(false);
        
    let is_websocket = headers.get(hyper::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);
        
    is_upgrade && is_websocket
}

pub async fn handle_websocket_upgrade<B>(
    req: Request<B>,
    upstream: &str,
) -> Result<Response<BoxBody<Bytes, crate::http_proxy::BoxError>>, hyper::Error> 
where
    B: hyper::body::Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    
    let path_and_query = req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or(req.uri().path());
    let upstream_url = format!("http://{}{}", upstream, path_and_query);

    debug!("🕸️ Forwarding WS upgrade to: {}", upstream_url);

    let client = reqwest::Client::new();
    let reqwest_method = reqwest::Method::from_bytes(req.method().as_str().as_bytes()).unwrap_or(reqwest::Method::GET);
    
    let mut upstream_req = client.request(reqwest_method, &upstream_url);

    // Copy original headers
    for (name, value) in req.headers().iter() {
        if name.as_str().to_lowercase() != "host" {
            if let Ok(v) = value.to_str() {
                upstream_req = upstream_req.header(name.as_str(), v);
            }
        }
    }

    // Must be empty body for an upgrade
    let upstream_res = match upstream_req.send().await {
        Ok(res) => res,
        Err(e) => {
            error!("Upstream WS proxy failed: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(crate::http_proxy::full(Bytes::from("WS Gateway Error")))
                .unwrap());
        }
    };

    if upstream_res.status() != reqwest::StatusCode::SWITCHING_PROTOCOLS {
        error!("Upstream failed to switch protocols: status {}", upstream_res.status());
        let mut builder = Response::builder().status(upstream_res.status().as_u16());
        for (k, v) in upstream_res.headers() {
            builder = builder.header(k.as_str(), v.as_bytes());
        }
        return Ok(builder.body(crate::http_proxy::full(Bytes::new())).unwrap());
    }

    let mut builder = Response::builder().status(StatusCode::SWITCHING_PROTOCOLS);
    for (name, value) in upstream_res.headers() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }
    let client_res = builder.body(crate::http_proxy::full(Bytes::new())).unwrap();

    // Spawn a task to handle the bidirectional pump once the connections are upgraded
    tokio::spawn(async move {
        tokio::task::yield_now().await;

        let client_upgraded = match hyper::upgrade::on(req).await {
            Ok(upgraded) => upgraded,
            Err(e) => {
                error!("Client WebSocket upgrade error: {}", e);
                return;
            }
        };

        let upstream_upgraded = match upstream_res.upgrade().await {
            Ok(upgraded) => upgraded,
            Err(e) => {
                error!("Upstream WebSocket upgrade error: {}", e);
                return;
            }
        };

        let mut client_io = TokioIo::new(client_upgraded);
        let mut upstream_io = upstream_upgraded;

        crate::metrics::increment_websocket_connections();
        
        debug!("WebSocket connection established. Starting bidirectional pump.");

        match tokio::io::copy_bidirectional(&mut client_io, &mut upstream_io).await {
            Ok((from_client, from_upstream)) => {
                debug!("WebSocket connection closed. Client sent {} bytes, upstream sent {} bytes", from_client, from_upstream);
            }
            Err(e) => {
                debug!("WebSocket connection closed with error: {}", e);
            }
        }
        
        crate::metrics::decrement_websocket_connections();
    });

    Ok(client_res)
}
