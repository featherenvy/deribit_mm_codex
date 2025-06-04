pub mod exchange;

#[cfg(test)]
mod tests {
    use super::exchange::DeribitClient;

    #[tokio::test]
    async fn client_connects() {
        let mut client = DeribitClient::new();
        // We only test that the connect function returns an error or ok without panicking
        let _ = client.connect_ws().await.err();
    }
}
