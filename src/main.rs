// Add dependencies to Cargo.toml
[dependencies]
reqwest = "0.11"
tokio = { version = "1", features = ["full"] }

// main.rs
#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let url = "https://news.ycombinator.com/threads?id=fragmede";
    let resp = reqwest::get(url).await?;
    let body = resp.text().await?;
    println!("body = {:?}", body);
    Ok(())
}
