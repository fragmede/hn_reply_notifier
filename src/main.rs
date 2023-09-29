extern crate reqwest;
extern crate scraper;
extern crate rusqlite;
extern crate notify_rust;
extern crate rodio;
extern crate chrono;

use chrono::Local;
use scraper::{Html, Selector};
use rusqlite::{params, Connection, Result};
use notify_rust::Notification;
use std::io::BufReader;
use std::fs::File;
use rodio::{Decoder, OutputStream, source::Source};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let now = Local::now();
        println!("Checking for new comments at {}", now.format("%Y-%m-%d %H:%M:%S"));

        let url = "https://news.ycombinator.com/threads?id=fragmede";
        let resp = reqwest::get(url).await?;
        let body = resp.text().await?;

        let fragment = Html::parse_document(&body);
        let comment_selector = Selector::parse(".commtext").unwrap();
        let author_selector = Selector::parse(".hnuser").unwrap();

        let conn = Connection::open("comments.db")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS comments (
                  id INTEGER PRIMARY KEY,
                  text TEXT NOT NULL
                  )",
            params![],
        )?;

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();

        for comment in fragment.select(&comment_selector) {

            let comment_text = comment.text().collect::<String>();

            // Navigate to the parent 'td' of the comment
            let parent_td = comment.ancestor_nodes().find(|node| {
                node.value().as_element().is_some() && node.value().as_element().unwrap().name.local.as_ref() == "td"
            });

            // Navigate to the sibling 'td' containing the author information
            let sibling_td = parent_td.and_then(|node| node.prev_sibling());

            // Extract the author's username
            let author = sibling_td.and_then(|node| {
                node.children().find(|child| {
                    child.value().as_element().is_some() && child.value().as_element().unwrap().name.local == "a"
                })
            }).and_then(|node| {
                node.text()
            }).unwrap_or(String::from("Unknown"));

            // Skip if the author is you
            if author == "fragmede" {
                continue;
            }

            let mut stmt = conn.prepare("SELECT id FROM comments WHERE text = ?1")?;
            let comment_exists: Result<i32> = stmt.query_row(params![comment_text], |row| row.get(0));

            if comment_exists.is_err() {
                conn.execute(
                    "INSERT INTO comments (text) VALUES (?1)",
                    params![comment_text],
                )?;
                println!("New comment: {}", comment_text);

                let first_10_words: String = comment_text.split_whitespace().take(10).collect::<Vec<&str>>().join(" ");
                Notification::new()
                    .summary("New Reply on Hacker News")
                    .body(&first_10_words)
                    .show()?;

                let file = BufReader::new(File::open("sound.mp3").unwrap());
                let source = Decoder::new(file).unwrap();
                if let Err(e) = stream_handle.play_raw(source.convert_samples()) {
                    eprintln!("Error playing sound: {}", e);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(60 * 5));
    }
}
