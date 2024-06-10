use clap::{Arg, App};
use dirs;
use notify_rust::Notification;
use rodio::{Decoder, OutputStream, source::Source};
use rusqlite::{params, Connection, Result};
use serde_json::Value;
use std::fs;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::thread::sleep;
use std::time::Duration;

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

	let username = matches
		.value_of("username")
		.map(|u| u.to_string())
		.unwrap_or_else(|| {
			let home_dir = dirs::home_dir().expect("Could not get home directory");
			let config_path = home_dir.join(".hackernews_comments");
			fs::read_to_string(config_path).ok()
				.map(|s| s.trim().to_string())
				.unwrap_or_else(|| {
					eprintln!("Username not set. Aborting.");
					std::process::abort();
				})
		});

	println!("Username checking for is {}", username);

    let conn = Connection::open("comments-id-only.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY,
            comment_id INTEGER NOT NULL
        )",
        params![],
    )?;

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    let url = "https://hacker-news.firebaseio.com/v0/user/fragmede.json";
    let profile_response = reqwest::get(url).await?;
    let body = profile_response.text().await?;
    let profile: Value = serde_json::from_str(&body)?;

	loop {
		println!("Getting: ");
		io::stdout().flush()?;  // Manually flush the standard output
		for x in 0..100 {
			let url = format!("https://hacker-news.firebaseio.com/v0/item/{}.json", profile["submitted"][x]);
			print!("\r{} ", profile["submitted"][x]);
			io::stdout().flush()?;  // Manually flush the standard output
			let resp = reqwest::get(url).await?;
			let body = resp.text().await?;
			let w: Value = serde_json::from_str(&body)?;

			if let Some(kids_wrapped_array) = w.get("kids") {
				if kids_wrapped_array.is_array() {
					let array: Vec<i64> = kids_wrapped_array.as_array()
						.unwrap()
						.iter()
						.map(|x| x.as_i64().unwrap())
						.collect();
					for comment_id in array {
						let mut stmt = conn.prepare("SELECT id FROM comments WHERE comment_id = ?1")?;
						let comment_exists: Result<i32> = stmt.query_row(params![comment_id], |row| row.get(0));

						if comment_exists.is_err() {
							println!("here2");
							conn.execute(
								"INSERT INTO comments (comment_id) VALUES (?1)",
								params![comment_id],
							)?;
							let comment_url = format!("https://hacker-news.firebaseio.com/v0/item/{}.json", &comment_id);
							let comment_response = reqwest::get(comment_url).await?;
							let comment_body= comment_response.text().await?;
							let comment_json: Value = serde_json::from_str(&comment_body)?;
							let comment_text = format!("{}",comment_json["text"]);
							println!("\nhttps://news.ycombinator.com/context?id={}\nNew comment.", &comment_id);

							if let Some(by) = comment_json.get("by").and_then(|by| by.as_str()) {
								if by == username {
									println!("cont");
									continue;
								}
							}
							println!("{}", comment_text);
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
				}
			}
			sleep(Duration::from_millis(10));

		}
		println!("Done. Sleeping...");
		sleep(Duration::from_secs(300));
	}
}
