use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    aegis_proxy::bootstrap::bootstrap().await
}
