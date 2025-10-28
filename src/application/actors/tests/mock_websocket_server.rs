use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct MockWebSocketConnection {
    pub auth_token: Option<String>,
    pub connected: bool,
}

impl MockWebSocketConnection {
    pub fn new() -> Self {
        Self {
            auth_token: None,
            connected: true,
        }
    }

    pub fn auth_token(&self) -> Option<String> {
        self.auth_token.clone()
    }

    pub async fn send_message(&mut self, _message: &str) {
        // Mock implementation
    }

    pub async fn close(&mut self) {
        self.connected = false;
    }
}

pub struct MockWebSocketServer {
    port: u16,
    listener: Option<Arc<TcpListener>>,
    connections: Arc<Mutex<VecDeque<MockWebSocketConnection>>>,
}

impl MockWebSocketServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            listener: None,
            connections: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn start(&mut self) -> SocketAddr {
        let addr: SocketAddr = format!("127.0.0.1:{}", self.port).parse().unwrap();
        let listener = Arc::new(TcpListener::bind(addr).await.unwrap());
        let local_addr = listener.local_addr().unwrap();
        self.listener = Some(listener.clone());

        let connections = Arc::clone(&self.connections);
        tokio::spawn(async move {
            loop {
                if let Ok((_, _peer_addr)) = listener.accept().await {
                    let mut conns = connections.lock().await;
                    conns.push_back(MockWebSocketConnection::new());
                }
            }
        });

        local_addr
    }

    pub async fn stop(&mut self) {
        self.listener = None;
    }

    pub async fn send_price(&mut self, _product_id: &str, _price: &str) -> Result<(), String> {
        Ok(())
    }

    pub async fn send_malformed_json(&mut self) -> Result<(), String> {
        Ok(())
    }

    pub async fn close_connection(&mut self) -> Result<(), String> {
        Ok(())
    }

    pub async fn next_connection(&mut self) -> Option<MockWebSocketConnection> {
        let mut conns = self.connections.lock().await;
        conns.pop_front()
    }

    pub async fn send_message(&mut self, _message: &str) -> Result<(), String> {
        Ok(())
    }

    pub async fn send_binary_frame(&mut self, _data: &[u8]) -> Result<(), String> {
        Ok(())
    }

    pub async fn send_raw_message(&mut self, _message: &str) -> Result<(), String> {
        Ok(())
    }

    pub async fn set_reject_connections(&mut self, _reject: bool) -> Result<(), String> {
        Ok(())
    }

    pub async fn set_failure_mode(&mut self, _enabled: bool) -> Result<(), String> {
        Ok(())
    }
}
