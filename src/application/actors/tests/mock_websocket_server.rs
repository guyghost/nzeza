use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct MockWebSocketConnection {
    pub auth_token: Option<String>,
    pub connected: bool,
    pub message_queue: Arc<Mutex<VecDeque<String>>>,
}

impl MockWebSocketConnection {
    pub fn new() -> Self {
        Self {
            auth_token: None,
            connected: true,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn auth_token(&self) -> Option<String> {
        self.auth_token.clone()
    }

    pub async fn send_message(&mut self, message: &str) {
        let mut queue = self.message_queue.lock().await;
        queue.push_back(message.to_string());
    }

    pub async fn receive_message(&mut self) -> Option<String> {
        let mut queue = self.message_queue.lock().await;
        queue.pop_front()
    }

    pub async fn close(&mut self) {
        self.connected = false;
    }
}

pub struct MockWebSocketServer {
    port: u16,
    listener: Option<Arc<TcpListener>>,
    connections: Arc<Mutex<VecDeque<MockWebSocketConnection>>>,
    reject_connections: Arc<Mutex<bool>>,
    failure_mode: Arc<Mutex<bool>>,
    message_queue: Arc<Mutex<VecDeque<String>>>,
}

impl MockWebSocketServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            listener: None,
            connections: Arc::new(Mutex::new(VecDeque::new())),
            reject_connections: Arc::new(Mutex::new(false)),
            failure_mode: Arc::new(Mutex::new(false)),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn start(&mut self) -> SocketAddr {
        let addr: SocketAddr = format!("127.0.0.1:{}", self.port).parse().unwrap();
        let listener = Arc::new(TcpListener::bind(addr).await.unwrap());
        let local_addr = listener.local_addr().unwrap();
        self.listener = Some(listener.clone());

        let connections = Arc::clone(&self.connections);
        let reject_connections = Arc::clone(&self.reject_connections);
        let message_queue = Arc::clone(&self.message_queue);

        tokio::spawn(async move {
            loop {
                if let Ok((_, _peer_addr)) = listener.accept().await {
                    let reject = *reject_connections.lock().await;
                    if !reject {
                        let mut conn = MockWebSocketConnection::new();
                        // Send any queued messages to the new connection
                        let mut queue = message_queue.lock().await;
                        while let Some(msg) = queue.pop_front() {
                            conn.send_message(&msg).await;
                        }
                        let mut conns = connections.lock().await;
                        conns.push_back(conn);
                    }
                }
            }
        });

        local_addr
    }

    pub async fn stop(&mut self) {
        self.listener = None;
    }

    pub async fn send_price(&mut self, product_id: &str, price: &str) -> Result<(), String> {
        let message = format!(r#"{{"product_id":"{}","price":"{}","timestamp":"2025-10-28T12:00:00Z"}}"#, product_id, price);
        let mut queue = self.message_queue.lock().await;
        queue.push_back(message);
        Ok(())
    }

    pub async fn send_malformed_json(&mut self) -> Result<(), String> {
        let malformed_messages = vec![
            r#"{"product_id": "BTC-USD", "price": "45000.50", invalid"#,
            r#"{"product_id": "ETH-USD" "price": "3200.75"}"#,
            r#"{product_id: "SOL-USD", price: "95.25"}"#,
            r#"{"product_id": "DOGE-USD", "price": 45000.50"#,
            r#"not json at all"#,
            r#""#,
        ];
        let mut queue = self.message_queue.lock().await;
        for msg in malformed_messages {
            queue.push_back(msg.to_string());
        }
        Ok(())
    }

    pub async fn close_connection(&mut self) -> Result<(), String> {
        let mut conns = self.connections.lock().await;
        if let Some(mut conn) = conns.pop_front() {
            conn.close().await;
        }
        Ok(())
    }

    pub async fn next_connection(&mut self) -> Option<MockWebSocketConnection> {
        let mut conns = self.connections.lock().await;
        conns.pop_front()
    }

    pub async fn send_message(&mut self, message: &str) -> Result<(), String> {
        let mut queue = self.message_queue.lock().await;
        queue.push_back(message.to_string());
        Ok(())
    }

    pub async fn send_binary_frame(&mut self, _data: &[u8]) -> Result<(), String> {
        // For simplicity, treat as malformed
        let mut queue = self.message_queue.lock().await;
        queue.push_back("binary_frame".to_string());
        Ok(())
    }

    pub async fn send_raw_message(&mut self, message: &str) -> Result<(), String> {
        let mut queue = self.message_queue.lock().await;
        queue.push_back(message.to_string());
        Ok(())
    }

    pub async fn set_reject_connections(&mut self, reject: bool) -> Result<(), String> {
        let mut reject_flag = self.reject_connections.lock().await;
        *reject_flag = reject;
        Ok(())
    }

    pub async fn simulate_connection(&mut self) -> Result<(), String> {
        let conn = MockWebSocketConnection::new();
        let mut conns = self.connections.lock().await;
        conns.push_back(conn);
        Ok(())
    }
