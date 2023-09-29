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

            // Navigate to the parent 'tr' of the comment
            let parent_tr = comment.ancestors().find(|node| {
                node.value().as_element().is_some() && node.value().as_element().unwrap().name.local.as_ref() == "tr"
            });

            // Navigate to the sibling 'td' containing the author information
            let sibling_td = parent_tr.and_then(|node| node.prev_sibling());

            // Extract the author's username
            let author = sibling_td.and_then(|node| {
                node.children().find(|child| {
                    child.value().as_element().is_some() && child.value().as_element().unwrap().name.local.as_ref() == "span"
                }).and_then(|span| {
                    span.children().find(|child| {
                        child.value().as_element().is_some() && child.value().as_element().unwrap().name.local.as_ref() == "a"
                    })
                })
            }).and_then(|node| {
                Some(node.children().filter_map(|n| {
                    if let Some(text) = n.value().as_text() {
                        Some(text.to_string())
                    } else {
                        None
                    }
                }).collect::<String>())
            }).unwrap_or(String::from("Unknown"));

            if author == "fragmede" || author == "Unknown" {
                continue;
            }

            let mut stmt = conn.prepare("SELECT id FROM comments WHERE text = ?1")?;
            let comment_exists: Result<i32> = stmt.query_row(params![comment_text], |row| row.get(0));

            if comment_exists.is_err() {
                conn.execute(
                    "INSERT INTO comments (text) VALUES (?1)",
                    params![comment_text],
                    )?;
                println!("New comment from {}: {}", author, comment_text);

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
