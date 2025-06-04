use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tokio::net::TcpStream;
use reqwest::Client;
use std::time::Duration;
use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;
use anyhow::Result;

const WS_URL: &str = "wss://test.deribit.com/ws/api/v2";
const HTTP_URL: &str = "https://test.deribit.com/api/v2";
const RECONNECT_DELAY_SECS: u64 = 5;

pub struct DeribitClient {
    http: Client,
    ws: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl DeribitClient {
    pub fn new() -> Self {
        Self {
            http: Client::new(),
            ws: None,
        }
    }

    pub async fn connect_ws(&mut self) -> tokio_tungstenite::tungstenite::Result<()> {
        let (stream, _) = connect_async(WS_URL).await?;
        self.ws = Some(stream);
        Ok(())
    }

    async fn ensure_ws(&mut self) -> tokio_tungstenite::tungstenite::Result<()> {
        if self.ws.is_none() {
            self.connect_ws().await?;
        }
        Ok(())
    }

    pub async fn subscribe(&mut self, channels: &[&str]) -> Result<()> {
        self.ensure_ws().await?;
        let sub = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "public/subscribe",
            "params": {"channels": channels},
        });
        if let Some(ws) = &mut self.ws {
            ws.send(Message::Text(sub.to_string().into())).await?;
        }
        Ok(())
    }

    pub async fn next_ws_message(&mut self) -> Result<Value> {
        loop {
            match &mut self.ws {
                Some(ws) => match ws.next().await {
                    Some(Ok(Message::Text(text))) => {
                        return Ok(serde_json::from_str(&text)?);
                    }
                    Some(Ok(_)) => continue,
                    Some(Err(_)) | None => {
                        self.ws = None;
                        tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                        self.connect_ws().await?;
                        self.subscribe(&[]).await.ok();
                    }
                },
                None => {
                    tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                    self.connect_ws().await?;
                }
            }
        }
    }

    pub async fn rpc_private(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let resp = self.http.post(HTTP_URL).json(&body).send().await?;
        let val: Value = resp.json().await?;
        Ok(val)
    }
}
