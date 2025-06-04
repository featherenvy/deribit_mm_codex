use deribit_mm_codex::exchange::DeribitClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = DeribitClient::new();
    // Attempt a simple WebSocket connection and subscribe to ticker channel
    client.connect_ws().await?;
    client.subscribe(&["ticker.BTC-PERPETUAL.raw"]).await?;
    if let Ok(msg) = client.next_ws_message().await {
        println!("Received message: {}", msg);
    }
    Ok(())
}
