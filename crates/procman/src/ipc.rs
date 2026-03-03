use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

use crate::process::{ProcessConfig, ProcessInfo};

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Connection timeout")]
    Timeout,
    #[error("Daemon error: {0}")]
    Daemon(String),
}

/// JSON-RPC style messages sent from CLI to Daemon
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IpcRequest {
    Start { name: String, config: ProcessConfig },
    Stop { name: String },
    Restart { name: String, delay_ms: u64 },
    Reload { name: String, config: ProcessConfig },
    Delete { name: String },
    List,
    Status { name: String },
}

/// JSON-RPC style messages sent from Daemon to CLI
#[derive(Debug, Serialize, Deserialize)]
pub enum IpcResponse {
    Ok,
    Error(String),
    ProcessList(Vec<ProcessInfo>),
    ProcessStatus(Option<ProcessInfo>),
}

/// Resolve the default Unix domain socket path
pub fn default_socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("aegis-flow.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/aegis-flow.sock"))
}

/// IPC Server that listens for connections and delegates request handling
pub struct IpcServer {
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { socket_path: path.into() }
    }

    /// Run the server loop indefinitely
    /// Takes a request handler future/closure to process requests
    pub async fn serve<F, Fut>(&self, handler: F) -> Result<(), IpcError>
    where
        F: Fn(IpcRequest) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = IpcResponse> + Send + 'static,
    {
        // Cleanup old socket if exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let handler_clone = handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, handler_clone).await {
                            tracing::error!("IPC client handling error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("IPC accept error: {}", e);
                }
            }
        }
    }

    async fn handle_client<F, Fut>(mut stream: UnixStream, handler: F) -> Result<(), IpcError>
    where
        F: Fn(IpcRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = IpcResponse> + Send + 'static,
    {
        // Simple length-prefixed JSON protocol
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut data_buf = vec![0u8; len];
        stream.read_exact(&mut data_buf).await?;

        let request: IpcRequest = serde_json::from_slice(&data_buf)?;
        
        // Execute the handler
        let response = handler(request).await;

        let resp_bytes = serde_json::to_vec(&response)?;
        let resp_len = (resp_bytes.len() as u32).to_be_bytes();

        stream.write_all(&resp_len).await?;
        stream.write_all(&resp_bytes).await?;
        stream.flush().await?;

        Ok(())
    }

    pub fn cleanup(&self) {
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// IPC Client for communicating with the Daemon
pub struct IpcClient {
    socket_path: PathBuf,
    timeout: Duration,
}

impl IpcClient {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: path.into(),
            timeout: Duration::from_secs(5),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn send_command(&self, request: &IpcRequest) -> Result<IpcResponse, IpcError> {
        let connect_fut = UnixStream::connect(&self.socket_path);
        
        let mut stream = tokio::time::timeout(self.timeout, connect_fut)
            .await
            .map_err(|_| IpcError::Timeout)?
            // We use ? here to map IoError directly to IoError
            ?;

        let req_bytes = serde_json::to_vec(request)?;
        let req_len = (req_bytes.len() as u32).to_be_bytes();

        let write_fut = async {
            stream.write_all(&req_len).await?;
            stream.write_all(&req_bytes).await?;
            stream.flush().await?;
            Result::<(), std::io::Error>::Ok(())
        };

        tokio::time::timeout(self.timeout, write_fut)
            .await
            .map_err(|_| IpcError::Timeout)??;

        let read_fut = async {
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).await?;
            let len = u32::from_be_bytes(len_buf) as usize;

            let mut data_buf = vec![0u8; len];
            stream.read_exact(&mut data_buf).await?;
            Result::<Vec<u8>, std::io::Error>::Ok(data_buf)
        };

        let data_buf = tokio::time::timeout(self.timeout, read_fut)
            .await
            .map_err(|_| IpcError::Timeout)??;

        let response: IpcResponse = serde_json::from_slice(&data_buf)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    // Create a temporary path for the socket
    fn get_temp_socket() -> PathBuf {
        let temp_dir = std::env::temp_dir();
        temp_dir.join(format!("aegis-ipc-test-{}.sock", std::process::id()))
    }

    #[tokio::test]
    async fn test_ipc_communication() {
        let sock_path = get_temp_socket();
        
        // Ensure cleanup at end of test (or test failure)
        let sock_path_clone = sock_path.clone();
        
        // Keep track of received requests
        let received_req = Arc::new(Mutex::new(None));
        let received_clone = received_req.clone();
        
        let handler = move |req: IpcRequest| {
            let received = received_clone.clone();
            async move {
                *received.lock().await = Some(req);
                IpcResponse::Ok
            }
        };

        let server = IpcServer::new(&sock_path);
        
        // Spawn server in background
        let server_task = tokio::spawn(async move {
            let _ = server.serve(handler).await;
        });
        
        // Wait for server to bound socket
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Client connects and sends
        let client = IpcClient::new(&sock_path).with_timeout(Duration::from_secs(1));
        let req = IpcRequest::Stop { name: "test-app".to_string() };
        
        let response = client.send_command(&req).await.unwrap();
        
        match response {
            IpcResponse::Ok => {},
            _ => panic!("Expected Ok response"),
        }
        
        let received = received_req.lock().await.take().unwrap();
        match received {
            IpcRequest::Stop { name } => assert_eq!(name, "test-app"),
            _ => panic!("Expected Stop request"),
        }
        
        // Clean up
        server_task.abort();
        if sock_path_clone.exists() {
            let _ = std::fs::remove_file(sock_path_clone);
        }
    }

    #[tokio::test]
    async fn test_ipc_timeout() {
        let sock_path = get_temp_socket();
        // Don't start a server
        
        let client = IpcClient::new(&sock_path).with_timeout(Duration::from_millis(10));
        let req = IpcRequest::List;
        
        let result = client.send_command(&req).await;
        assert!(result.is_err());
        // Since there is no listener, connection will be refused instantly, or we timeout if firewall drops it. 
        // Either way it should be an error.
    }
}
