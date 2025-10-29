use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, mpsc};
use std::collections::VecDeque;
use tokio_tungstenite::accept_async;
use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;
use tokio_tungstenite::tungstenite::Message;

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

#[derive(Clone)]
pub struct MockWebSocketServer {
    port: u16,
    listener: Option<Arc<TcpListener>>,
    connections: Arc<Mutex<VecDeque<MockWebSocketConnection>>>,
    reject_connections: Arc<Mutex<bool>>,
    failure_mode: Arc<Mutex<bool>>,
    message_queue: Arc<Mutex<VecDeque<String>>>,
    active: Arc<Mutex<bool>>,
    message_tx: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
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
            active: Arc::new(Mutex::new(false)),
            message_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&mut self) -> SocketAddr {
        let addr: SocketAddr = format!("127.0.0.1:{}", self.port).parse().unwrap();
        let listener = Arc::new(TcpListener::bind(addr).await.unwrap());
        let local_addr = listener.local_addr().unwrap();
        self.listener = Some(listener.clone());

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        *self.message_tx.lock().await = Some(tx);

        let (broadcast_tx, _) = tokio::sync::broadcast::channel::<String>(100);

        let connections = Arc::clone(&self.connections);
        let reject_connections = Arc::clone(&self.reject_connections);
        let message_queue = Arc::clone(&self.message_queue);
        let active = Arc::clone(&self.active);

        *active.lock().await = true;

        let listener_clone = listener.clone();
        let broadcast_tx_clone = broadcast_tx.clone();
        let active_clone = active.clone();
        let connections_clone = connections.clone();
        let message_queue_clone = message_queue.clone();
        let reject_connections_clone = reject_connections.clone();

        tokio::spawn(async move {
            loop {
                // Check if server is still active
                if !*active_clone.lock().await {
                    break;
                }

                match tokio::time::timeout(std::time::Duration::from_millis(100), listener_clone.accept()).await {
                    Ok(Ok((stream, _peer_addr))) => {
                        let reject = *reject_connections_clone.lock().await;
                        if reject {
                            continue;
                        }

                        let conns = Arc::clone(&connections_clone);
                        let msgs = Arc::clone(&message_queue_clone);
                        let bcast_tx = broadcast_tx_clone.clone();

                        tokio::spawn(async move {
                            // Handle WebSocket handshake
                            match accept_async(stream).await {
                                Ok(ws_stream) => {
                                    let (mut write, mut read) = ws_stream.split();
                                    let mut conn = MockWebSocketConnection::new();
                                    
                                    // Send any queued messages to the new connection
                                    let mut queue = msgs.lock().await;
                                    let queued_msgs: Vec<String> = queue.drain(..).collect();
                                    drop(queue);
                                    
                                    for msg in queued_msgs {
                                        let _ = write.send(Message::Text(msg)).await;
                                    }
                                    
                                    // Store connection
                                    let mut conns = conns.lock().await;
                                    conns.push_back(conn);
                                    drop(conns);

                                    // Subscribe to broadcast messages and forward them
                                    let mut subscribe = bcast_tx.subscribe();
                                    loop {
                                        tokio::select! {
                                            // Receive broadcast message and send through WebSocket
                                            msg = subscribe.recv() => {
                                                if let Ok(msg_text) = msg {
                                                    let _ = write.send(Message::Text(msg_text)).await;
                                                } else {
                                                    break;
                                                }
                                            }
                                            // Read from client (keep connection alive)
                                            client_msg = read.next() => {
                                                if client_msg.is_none() {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    // WebSocket handshake failed
                                }
                            }
                        });
                    }
                    Ok(Err(_)) => {
                        // Accept error
                        break;
                    }
                    Err(_) => {
                        // Timeout - continue accepting
                    }
                }
            }
        });

        // Spawn message broadcaster - reads from channel and broadcasts to all clients
        let msg_queue_clone = message_queue.clone();
        let broadcast_tx_for_broadcaster = broadcast_tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                // Send to broadcast channel
                let _ = broadcast_tx_for_broadcaster.send(msg.clone());
                // Also store in queue for late-joining clients
                let mut queue = msg_queue_clone.lock().await;
                queue.push_back(msg);
            }
        });

        local_addr
    }

    pub async fn stop(&mut self) {
        *self.active.lock().await = false;
        self.listener = None;
    }

    pub async fn send_price(&mut self, product_id: &str, price: &str) -> Result<(), String> {
        let message = format!(r#"{{"product_id":"{}","price":"{}","timestamp":"2025-10-28T12:00:00Z"}}"#, product_id, price);
        if let Some(tx) = self.message_tx.lock().await.as_ref() {
            let _ = tx.send(message);
        } else {
            let mut queue = self.message_queue.lock().await;
            queue.push_back(message);
        }
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
        for msg in malformed_messages {
            if let Some(tx) = self.message_tx.lock().await.as_ref() {
                let _ = tx.send(msg.to_string());
            } else {
                let mut queue = self.message_queue.lock().await;
                queue.push_back(msg.to_string());
            }
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
        if let Some(tx) = self.message_tx.lock().await.as_ref() {
            let _ = tx.send(message.to_string());
        } else {
            let mut queue = self.message_queue.lock().await;
            queue.push_back(message.to_string());
        }
        Ok(())
    }

    pub async fn send_binary_frame(&mut self, _data: &[u8]) -> Result<(), String> {
        // For simplicity, treat as malformed
        if let Some(tx) = self.message_tx.lock().await.as_ref() {
            let _ = tx.send("binary_frame".to_string());
        } else {
            let mut queue = self.message_queue.lock().await;
            queue.push_back("binary_frame".to_string());
        }
        Ok(())
    }

    pub async fn send_raw_message(&mut self, message: &str) -> Result<(), String> {
        if let Some(tx) = self.message_tx.lock().await.as_ref() {
            let _ = tx.send(message.to_string());
        } else {
            let mut queue = self.message_queue.lock().await;
            queue.push_back(message.to_string());
        }
        Ok(())
    }

    pub async fn set_reject_connections(&mut self, reject: bool) -> Result<(), String> {
        let mut reject_flag = self.reject_connections.lock().await;
        *reject_flag = reject;
        Ok(())
    }

    pub async fn set_failure_mode(&mut self, failure: bool) -> Result<(), String> {
        let mut failure_flag = self.failure_mode.lock().await;
        *failure_flag = failure;
        Ok(())
    }

    pub async fn simulate_connection(&mut self) -> Result<(), String> {
        let conn = MockWebSocketConnection::new();
        let mut conns = self.connections.lock().await;
        conns.push_back(conn);
        Ok(())
    }
}