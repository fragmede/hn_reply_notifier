use chrono::Local;
use scraper::{Html, Selector};
use rusqlite::{params, Connection, Result};
use notify_rust::Notification;
use std::io::BufReader;
use std::fs::File;
use rodio::{Decoder, OutputStream, source::Source};
use string_join::Join;
use std::fs;
use clap::{Arg, App};

async fn process_page(url: &str, username: &str, conn: &Connection, stream_handle: &OutputStreamHandle) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let resp = reqwest::get(url).await?;
    let body = resp.text().await?;
    let fragment = Html::parse_document(&body);
    let comment_selector = Selector::parse(".commtext").unwrap();
    let more_selector = Selector::parse("a.morelink").unwrap();

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

        if author == username {
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

        // Look for the "More" link to get the next page ID
        let next_page_id = fragment.select(&more_selector)
            .filter_map(|node| node.value().attr("href"))
            .next()
            .and_then(|href| {
                let parts: Vec<&str> = href.split('=').collect();
                parts.get(2).map(|&s| s.to_string())
            });

        Ok(next_page_id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("Hacker News Comment Notifier")
        .version("1.0")
        .author("Your Name")
        .about("Notifies you of new comments on your Hacker News posts")
        .arg(Arg::with_name("username")
             .short("u")
             .long("username")
             .value_name("USERNAME")
             .help("Sets a custom username")
             .takes_value(true))
        .get_matches();

    let username = if let Some(u) = matches.value_of("username") {
        u.to_string()
    } else {
        let home_dir = dirs::home_dir().expect("Could not get home directory");
        let config_path = home_dir.join(".hackernews_comments");
        fs::read_to_string(config_path)
            .map(|s| s.trim().to_string())
            .ok_or("Username not specified in config file or command line")?
    };

    if username.is_empty() {
        return Err("Username cannot be empty".into());
    }

    loop {
        let now = Local::now();
        println!("Checking for new comments at {}", now.format("%Y-%m-%d %H:%M:%S"));
        let mut next_page_id: Option<String> = None;
        for _ in 0..4 {  // Loop up to 4 times
            let url = if let Some(ref id) = next_page_id {
                format!("https://news.ycombinator.com/threads?id={}&next={}", username, id)
            } else {
                format!("https://news.ycombinator.com/threads?id={}", username)
            };

            next_page_id = process_page(&url, &username, &conn, &stream_handle).await?;
            if next_page_id.is_none() {
                break;  // Exit the loop if there's no "More" link
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(60 * 5));
    }
}
