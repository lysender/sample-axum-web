#![allow(unused)] // For beginning only

use anyhow::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let client = httpc_test::new_client("http://localhost:8080")?;

    client.do_get("/hello?name=Burpy").await?.print().await?;
    client.do_get("/hello2/Binky").await?.print().await?;
    client.do_get("/manifest.json").await?.print().await?;

    let login_request = client.do_post(
        "/api/login",
        json!({
            "username": "root",
            "password": "password"
        })
    );
    login_request.await?.print().await?;

    let req_create_ticket = client.do_post(
        "/api/tickets",
        json!({
            "title": "Ticket AAA",
        })
    );
    req_create_ticket.await?.print().await?;
    client.do_delete("/api/tickets/1").await?.print().await?;
    client.do_get("/api/tickets").await?.print().await?;

    Ok(())
}
