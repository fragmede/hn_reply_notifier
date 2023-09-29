extern crate reqwest;
extern crate scraper;
extern crate rusqlite;

use scraper::{Html, Selector};
use rusqlite::{params, Connection, Result};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://news.ycombinator.com/threads?id=fragmede";
    let resp = reqwest::get(url).await?;
    let body = resp.text().await?;

    let fragment = Html::parse_document(&body);
    let comment_selector = Selector::parse(".commtext").unwrap();

    let conn = Connection::open("comments.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS comments (
                  id INTEGER PRIMARY KEY,
                  text TEXT NOT NULL
                  )",
        params![],
    )?;

    for comment in fragment.select(&comment_selector) {
        let comment_text = comment.text().collect::<String>();
        conn.execute(
            "INSERT INTO comments (text) VALUES (?1)",
            params![comment_text],
        )?;
        println!("Saved comment: {}", comment_text);
    }

    Ok(())
}
