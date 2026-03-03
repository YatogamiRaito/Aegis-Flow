use crate::config::StreamConfig;
use crate::proxy_protocol::ProxyHeader;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

/// TCP Stream Proxy Server
pub struct StreamProxyServer {
    config: StreamConfig,
}

impl StreamProxyServer {
    pub fn new(config: StreamConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.listen).await?;
        info!("🎯 L4 TCP Stream Proxy listening on {}", self.config.listen);

        loop {
            match listener.accept().await {
                Ok((client_stream, client_addr)) => {
                    debug!("📥 New TCP stream connection from: {}", client_addr);
                    
                    let upstream_addr = self.config.proxy_pass.clone();
                    
                    let config_clone = self.config.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_tcp_stream(client_stream, client_addr, config_clone).await {
                            warn!("⚠️ TCP Stream proxy error for {}: {}", client_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("❌ TCP Accept error: {}", e);
                }
            }
        }
    }
}

async fn handle_tcp_stream(
    mut client_stream: TcpStream,
    mut client_addr: std::net::SocketAddr,
    config: StreamConfig,
) -> Result<()> {
    let mut initial_buffer = Vec::new();
    let upstream_addr = &config.proxy_pass;

    if config.proxy_protocol {
        let mut buf = [0u8; 1024]; // Max proxy header size
        match client_stream.read(&mut buf).await {
            Ok(0) => return Ok(()),
            Ok(n) => {
                // Try to parse v1 or v2
                if let Ok(Some((header, bytes_read))) = ProxyHeader::parse_v2(&buf[..n]) {
                    client_addr = header.source_addr;
                    debug!("🕵️ PROXY Protocol v2 decoded. Real client IP: {}", client_addr);
                    initial_buffer.extend_from_slice(&buf[bytes_read..n]);
                } else if let Ok(Some((header, bytes_read))) = ProxyHeader::parse_v1(&buf[..n]) {
                    client_addr = header.source_addr;
                    debug!("🕵️ PROXY Protocol v1 decoded. Real client IP: {}", client_addr);
                    initial_buffer.extend_from_slice(&buf[bytes_read..n]);
                } else {
                    // Could not parse, disconnect early or treat as raw data
                    warn!("⚠️ Invalid PROXY Protocol header received from: {}", client_addr);
                    return Ok(());
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    // Attempt to connect to the upstream server
    let mut upstream_stream = match TcpStream::connect(upstream_addr).await {
        Ok(s) => s,
        Err(e) => {
            // If we can't connect upstream, close the client connection immediately
            error!("Failed to connect to TCP upstream {}: {}", upstream_addr, e);
            let _ = client_stream.shutdown().await;
            return Err(e.into());
        }
    };

    debug!("🔗 Established TCP connection to upstream {}", upstream_addr);

    // If we buffered data after stripping PROXY Protocol, send it now
    if !initial_buffer.is_empty() {
        upstream_stream.write_all(&initial_buffer).await?;
    }

    // Bidirectional copy between client and upstream
    match tokio::io::copy_bidirectional(&mut client_stream, &mut upstream_stream).await {
        Ok((from_client, from_upstream)) => {
            debug!(
                "🔌 TCP Stream closed. Client ({}) sent {} bytes, upstream sent {} bytes",
                client_addr, from_client, from_upstream
            );
        }
        Err(e) => {
            debug!("⚠️ TCP Stream bidirectional pump error for {}: {}", client_addr, e);
        }
    }

    Ok(())
}
