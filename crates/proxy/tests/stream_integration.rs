use aegis_proxy::config::{ProxyConfig, StreamConfig, StreamProtocol};
use aegis_proxy::stream_proxy::StreamProxyServer;
use aegis_proxy::udp_proxy::UdpProxyServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_tcp_stream_proxy() {
    let mock_upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = mock_upstream.local_addr().unwrap();

    let stream_cfg = StreamConfig {
        listen: "127.0.0.1:0".to_string(),
        protocol: StreamProtocol::Tcp,
        proxy_pass: upstream_addr.to_string(),
        proxy_timeout: None,
        proxy_responses: 0,
        proxy_protocol: false,
        servers: vec![],
    };

    let proxy = StreamProxyServer::new(stream_cfg.clone());
    
    // Bind to test a random free port gracefully but we can't easily extract the auto-bound port
    // from StreamProxyServer natively without exposing its listener handle. 
    // We'll trust it binds successfully if we pass it an explicit port.
    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    
    // We will drop this listener and let StreamProxyServer bind to its port immediately
    drop(proxy_listener);
    
    let mut actual_cfg = stream_cfg;
    actual_cfg.listen = proxy_addr.to_string();
    let final_proxy = StreamProxyServer::new(actual_cfg);
    
    tokio::spawn(async move {
        // Will run forever
        let _ = final_proxy.run().await;
    });

    // Mock upstream logic
    tokio::spawn(async move {
        if let Ok((mut upstream_sock, _)) = mock_upstream.accept().await {
            let mut buf = [0u8; 1024];
            if let Ok(n) = upstream_sock.read(&mut buf).await {
                // Echo appended with upstream magic
                let mut reply = b"UPSTREAM_ECHO_".to_vec();
                reply.extend_from_slice(&buf[..n]);
                upstream_sock.write_all(&reply).await.unwrap();
            }
        }
    });

    // Wait for the stream server to bind
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client to proxy
    let mut client = timeout(Duration::from_secs(1), TcpStream::connect(proxy_addr))
        .await
        .expect("Client connect timed out")
        .expect("Client connect failed");
        
    client.write_all(b"HELLO_Aegis").await.unwrap();
    
    let mut response = [0u8; 128];
    let n = timeout(Duration::from_secs(1), client.read(&mut response))
        .await
        .expect("Client read timed out")
        .expect("Client read failed");
        
    let response_str = std::str::from_utf8(&response[..n]).unwrap();
    assert_eq!(response_str, "UPSTREAM_ECHO_HELLO_Aegis");
}

#[tokio::test]
async fn test_udp_stream_proxy() {
    let mock_upstream = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = mock_upstream.local_addr().unwrap();

    let proxy_listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    drop(proxy_listener);

    let stream_cfg = StreamConfig {
        listen: proxy_addr.to_string(),
        protocol: StreamProtocol::Udp,
        proxy_pass: upstream_addr.to_string(),
        proxy_timeout: None,
        proxy_responses: 1, // Expect 1 response before evicting
        proxy_protocol: false,
        servers: vec![],
    };

    let proxy = UdpProxyServer::new(stream_cfg);
    tokio::spawn(async move {
        let _ = proxy.run().await;
    });

    // Mock upstream echo service
    tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        if let Ok((n, src)) = mock_upstream.recv_from(&mut buf).await {
            // Note: `src` here is the proxy's ephemeral socket! Not the test client.
            let mut reply = b"UDP_ECHO_".to_vec();
            reply.extend_from_slice(&buf[..n]);
            mock_upstream.send_to(&reply, src).await.unwrap();
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let test_client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    test_client.send_to(b"UDP_HELLO", proxy_addr).await.unwrap();
    
    let mut response = [0u8; 128];
    let (n, src) = timeout(Duration::from_secs(1), test_client.recv_from(&mut response))
        .await
        .expect("UDP read timed out")
        .expect("UDP read failed");
        
    assert_eq!(src, proxy_addr); // Response should come from proxy, not upstream directly
    let response_str = std::str::from_utf8(&response[..n]).unwrap();
    assert_eq!(response_str, "UDP_ECHO_UDP_HELLO");
}

#[tokio::test]
async fn test_tcp_stream_proxy_protocol_v1() {
    let mock_upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = mock_upstream.local_addr().unwrap();

    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    drop(proxy_listener);

    let stream_cfg = StreamConfig {
        listen: proxy_addr.to_string(),
        protocol: StreamProtocol::Tcp,
        proxy_pass: upstream_addr.to_string(),
        proxy_timeout: None,
        proxy_responses: 0,
        proxy_protocol: true, // We have enabled PROXY protocol receiving!
        servers: vec![],
    };

    let proxy = StreamProxyServer::new(stream_cfg);
    tokio::spawn(async move {
        let _ = proxy.run().await;
    });

    // Mock upstream logic
    tokio::spawn(async move {
        if let Ok((mut upstream_sock, _)) = mock_upstream.accept().await {
            let mut buf = [0u8; 1024];
            if let Ok(n) = upstream_sock.read(&mut buf).await {
                let mut reply = b"UP_".to_vec();
                reply.extend_from_slice(&buf[..n]);
                upstream_sock.write_all(&reply).await.unwrap();
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client
    let mut client = timeout(Duration::from_secs(1), TcpStream::connect(proxy_addr))
        .await
        .unwrap()
        .unwrap();

    // Send PROXY protocol v1 followed by real data
    client.write_all(b"PROXY TCP4 198.51.100.22 203.0.113.7 35646 80\r\nPURE_PAYLOAD")
        .await.unwrap();

    let mut response = [0u8; 128];
    let n = timeout(Duration::from_secs(1), client.read(&mut response))
        .await
        .unwrap()
        .unwrap();

    // The upstream mock receives ONLY "PURE_PAYLOAD", replies "UP_PURE_PAYLOAD"
    let response_str = std::str::from_utf8(&response[..n]).unwrap();
    assert_eq!(response_str, "UP_PURE_PAYLOAD");
}
