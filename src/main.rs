use chrono::Local;
use scraper::{Html, Selector};
use rusqlite::{params, Connection, Result};
use notify_rust::Notification;
use std::io::BufReader;
use std::fs::File;
use rodio::{Decoder, OutputStream, source::Source};
use string_join::Join;
use std::env;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read username from command line arguments or from a file
    let args: Vec<String> = env::args().collect();
    let username = if args.len() > 1 {
        args[1].clone()
    } else {
        let home_dir = dirs::home_dir().expect("Could not get home directory");
        let config_path = home_dir.join(".hackernews_comments");
        fs::read_to_string(config_path)
            .unwrap_or(String::from("fragmede"))
            .trim()
            .to_string()
    };

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

            let author = comment.ancestors().find(|node| {
                node.value().as_element().is_some() && node.value().as_element().unwrap().name.local.as_ref() == "td"
            }).and_then(|td| {
                td.children().find(|child| {
                    child.value().as_element().is_some() && child.value().as_element().unwrap().name.local.as_ref() == "div"
                })
            }).and_then(|div| {
                div.children().find(|child| {
                    child.value().as_element().is_some() && child.value().as_element().unwrap().name.local.as_ref() == "span"
                })
            }).and_then(|span| {
                span.children().find(|child| {
                    child.value().as_element().is_some() && child.value().as_element().unwrap().name.local.as_ref() == "a"
                })
            }).and_then(|a| {
                Some(a.children().filter_map(|n| {
                    if let Some(text) = n.value().as_text() {
                        Some(text.to_string())
                    } else {
                        None
                    }
                }).collect::<String>())
            }).unwrap_or(String::from("Unknown"));

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
                println!("New comment from {}: {}", author, comment_text);

                let first_10_words: String = " ".join(comment_text.split_whitespace().take(10));

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
