use chrono::Local;
use clap::{Arg, App};
use dirs;
use notify_rust::Notification;
use rodio::{Decoder, OutputStream, source::Source};
use rusqlite::{params, Connection, Result};
use scraper::{Html, Selector};
use std::error::Error;
use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::thread::sleep;
use std::time::Duration;
use reqwest::StatusCode;

async fn get_page(url: &str, username: &str, conn: &Connection, stream_handle: &rodio::OutputStreamHandle) -> Result<Option<String>, Box<dyn Error>> {
    println!("Checking for new comments on {}", url);
    let resp = reqwest::get(url).await?;

	if let Err(e) = resp.error_for_status_ref() {
		match e.status() {
			Some(StatusCode::NOT_FOUND) => {
				eprintln!("Error: Resource not found (404)");
                return Err(Box::new(e));
			},
			Some(StatusCode::TOO_MANY_REQUESTS) => {
				eprintln!("Error: Rate limited (429)");
                return Err(Box::new(e));
			},
			Some(StatusCode::SERVICE_UNAVAILABLE) => {
				eprintln!("Error: pseudo Rate limited (503)");
                return Err(Box::new(e));
			},
			_ => {
				eprintln!("Error: some other err {}", e);
                return Err(Box::new(e));
			}
		}
	}
	let body = resp.text().await?;
	match process_page(&body, username, conn, stream_handle).await {
		Ok(Some(processed_body)) => Ok(Some(processed_body)),
		Ok(None) => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No content in processed page"))),
		Err(e) => {
			eprintln!("Error processing page: {}", e);
			Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Error parsing: {}", e))))
		}
	}

}

async fn process_page(body: &str, username: &str, conn: &Connection, stream_handle: &rodio::OutputStreamHandle) -> Result<Option<String>, Box<dyn std::error::Error>> {
    //let body = resp.text().await?;
    let fragment = Html::parse_document(&body);
    let comments_selector = Selector::parse(".athing").unwrap();
    let commtext_sel = Selector::parse(".commtext").unwrap();
    //let commhead_sel = Selector::parse(".comhead").unwrap();

	//for comment in fragment.select(&comment_selector) {
	for comment in fragment.select(&comments_selector) {
		let comment_text = comment.select(&commtext_sel).next().unwrap().text().collect::<String>();
		let author = comment.select(&Selector::parse(".hnuser").unwrap()).next().unwrap().text().collect::<String>();
		let comment_id = comment.value().attr("id").unwrap();

		//println!("\n\n\n\nComment Text: {}", comment_text);
		//println!("Author: {}", author); // author remains a String
        if author == username {
            //println!("Ignoring the reply you wrote.");
            continue;
        }

        let mut stmt = conn.prepare("SELECT id FROM comments WHERE text = ?1")?;
        let comment_exists: Result<i32> = stmt.query_row(params![comment_text], |row| row.get(0));

        if comment_exists.is_err() {
            conn.execute(
                "INSERT INTO comments (text) VALUES (?1)",
                params![comment_text],
                )?;
            println!("https://news.ycombinator.com/item?id={}\nNew comment from {}: {}", &comment_id, author, comment_text);

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
    let more_selector = Selector::parse("a.morelink").unwrap();
	let next_page_id = fragment.select(&more_selector)
		.filter_map(|node| {
			let href = node.value().attr("href");
			if href.is_none() {
				eprintln!("Node does not have href attribute");
			} else {
				//eprintln!("Node href attribute: {}", href.unwrap());
			}
			href
		})
	.next()
		.and_then(|href| {
			//eprintln!("Processing href: {}", href);
			let parts: Vec<&str> = href.split('=').collect();
			if parts.len() > 2 {
				Some(parts[2].to_string())
			} else {
				eprintln!("Unexpected href format: {}", href);
				None
			}
		});

	if next_page_id.is_none() {
		eprintln!("next_page_id could not be determined");
		return Err(Box::<dyn Error>::from("next_page_id could not be determined"));
	}
	Ok(next_page_id)
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

    let conn = Connection::open("comments.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS comments (
              id INTEGER PRIMARY KEY,
              text TEXT NOT NULL
              )",
        params![],
    )?;

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    loop {
        let now = Local::now();
        println!("Checking for new comments at {}", now.format("%Y-%m-%d %H:%M:%S"));

        let mut next_page_id = None;
        for _ in 0..12 {
            let url = match &next_page_id {
                None => format!("https://news.ycombinator.com/threads?id={}", username),
                Some(id) => format!("https://news.ycombinator.com/threads?id={}&next={}", username, id),
            };
			loop {
				match get_page(&url, &username, &conn, &stream_handle).await {
					Ok(id) => next_page_id = id,
					Err(e) => match e.downcast_ref::<reqwest::Error>() { // this doesn't actually work to get the StatusCode :(
						Some(http_err) => match http_err.status() {      // because we give it an ErrorKind::Other
							Some(status) => match status {
								StatusCode::OK=> {
									eprintln!("ok");
									break;
								},
								StatusCode::NOT_FOUND => {
									eprintln!("Error: Resource not found (404)");
									break;
								},
								StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => {
									// HN throws a 503 instead of a 429
									eprintln!("Error: rate limit");
									sleep(Duration::from_secs(3));
								},
								err => {
									eprintln!("http error: {}", err);
									std::process::abort();
								},
							},
							None => {
								eprintln!("other error");
								std::process::abort();
							},
						}
						None => {
							eprintln!("Error processing page: {}", e);
							break;
						},
					}
				}
				println!("Sleeping real quick to not hammer...");
				sleep(Duration::from_millis(200));
				break;
			}
        }
		println!("Sleeping for longer...");
        //sleep(Duration::from_mins(15));
        // from_mins not currently available: https://github.com/rust-lang/rust/issues/120301
        sleep(Duration::from_secs(60*15));
    }
}
