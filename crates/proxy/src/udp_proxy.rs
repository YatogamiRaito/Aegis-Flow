use crate::config::StreamConfig;
use crate::proxy_protocol::ProxyHeader;
use anyhow::Result;
use lru::LruCache;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

struct Session {
    upstream_socket: Arc<UdpSocket>,
    responses_expected: usize,
    responses_received: usize,
}

pub struct UdpProxyServer {
    config: StreamConfig,
}

impl UdpProxyServer {
    pub fn new(config: StreamConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<()> {
        let listener = Arc::new(UdpSocket::bind(&self.config.listen).await?);
        info!("🎯 L4 UDP Stream Proxy listening on {}", self.config.listen);

        let session_table: Arc<Mutex<LruCache<SocketAddr, Session>>> =
            Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(10000).unwrap())));

        let mut buf = [0u8; 65535]; // Max UDP datagram size

        loop {
            match listener.recv_from(&mut buf).await {
                Ok((len, mut client_addr)) => {
                    let mut data_start = 0;

                    if self.config.proxy_protocol {
                        // Attempt to parse out the header before forwarding payload
                        if let Ok(Some((header, bytes_read))) = ProxyHeader::parse_v2(&buf[..len]) {
                            client_addr = header.source_addr;
                            debug!(
                                "🕵️ UDP PROXY Protocol v2 decoded. Real client IP: {}",
                                client_addr
                            );
                            data_start = bytes_read;
                        } else if let Ok(Some((header, bytes_read))) =
                            ProxyHeader::parse_v1(&buf[..len])
                        {
                            client_addr = header.source_addr;
                            debug!(
                                "🕵️ UDP PROXY Protocol v1 decoded. Real client IP: {}",
                                client_addr
                            );
                            data_start = bytes_read;
                        } else {
                            warn!(
                                "⚠️ Invalid UDP PROXY Protocol header received from: {}",
                                client_addr
                            );
                            continue; // Drop the datagram
                        }
                    }

                    let data = buf[data_start..len].to_vec();
                    let upstream_addr = self.config.proxy_pass.clone();
                    let expected_responses = self.config.proxy_responses;
                    let listener_clone = listener.clone();
                    let sessions_clone = session_table.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_udp_datagram(
                            client_addr,
                            data,
                            upstream_addr,
                            expected_responses,
                            listener_clone,
                            sessions_clone,
                        )
                        .await
                        {
                            warn!("⚠️ UDP datagram handling error for {}: {}", client_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("❌ UDP Recv error: {}", e);
                }
            }
        }
    }
}

async fn handle_udp_datagram(
    client_addr: SocketAddr,
    data: Vec<u8>,
    upstream_addr: String,
    expected_responses: usize,
    listener: Arc<UdpSocket>,
    sessions: Arc<Mutex<LruCache<SocketAddr, Session>>>,
) -> Result<()> {
    let upstream_socket = {
        let mut session_lock = sessions.lock().await;

        if let Some(session) = session_lock.get(&client_addr) {
            session.upstream_socket.clone()
        } else {
            // Bind a new ephemeral socket for this client session
            let new_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
            new_socket.connect(&upstream_addr).await?;

            let session = Session {
                upstream_socket: new_socket.clone(),
                responses_expected: expected_responses,
                responses_received: 0,
            };

            session_lock.put(client_addr, session);

            // Spawn a task to listen for responses from this specific upstream socket
            if expected_responses > 0 {
                let sock_clone = new_socket.clone();
                let table_clone = sessions.clone();
                let listen_clone = listener.clone();

                tokio::spawn(async move {
                    pump_udp_responses(
                        sock_clone,
                        client_addr,
                        listen_clone,
                        table_clone,
                        expected_responses,
                    )
                    .await;
                });
            }

            new_socket
        }
    };

    // Forward the datagram to the upstream server
    upstream_socket.send(&data).await?;
    debug!(
        "📤 Forwarded {} byte UDP datagram from {} to {}",
        data.len(),
        client_addr,
        upstream_addr
    );

    // If we expect 0 responses, we can evict immediately. Fire and forget.
    if expected_responses == 0 {
        let mut session_lock = sessions.lock().await;
        session_lock.pop(&client_addr);
    }

    Ok(())
}

async fn pump_udp_responses(
    upstream_socket: Arc<UdpSocket>,
    client_addr: SocketAddr,
    listener: Arc<UdpSocket>,
    sessions: Arc<Mutex<LruCache<SocketAddr, Session>>>,
    expected_responses: usize,
) {
    let mut buf = [0u8; 65535];

    while let Ok(len) = upstream_socket.recv(&mut buf).await {
        // Forward the response back to the client using the main listening socket
        if let Err(e) = listener.send_to(&buf[..len], client_addr).await {
            warn!(
                "⚠️ Failed to send UDP response back to client {}: {}",
                client_addr, e
            );
            break;
        }

        debug!("📥 Forwarded {} byte UDP response to {}", len, client_addr);

        let mut remove = false;
        {
            let mut session_lock = sessions.lock().await;
            if let Some(session) = session_lock.get_mut(&client_addr) {
                session.responses_received += 1;
                if session.responses_received >= session.responses_expected {
                    remove = true;
                }
            }
        }

        if remove {
            let mut session_lock = sessions.lock().await;
            session_lock.pop(&client_addr);
            debug!(
                "🛑 Reached expected responses ({}), evicting UDP session for {}",
                expected_responses, client_addr
            );
            break;
        }
    }
}
