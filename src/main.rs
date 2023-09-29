extern crate reqwest;
extern crate scraper;

use scraper::{Html, Selector};

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let url = "https://news.ycombinator.com/threads?id=fragmede";
    let resp = reqwest::get(url).await?;
    let body = resp.text().await?;

    let fragment = Html::parse_document(&body);
    let comment_selector = Selector::parse(".commtext").unwrap();

    for comment in fragment.select(&comment_selector) {
        let comment_text = comment.text().collect::<String>();
        println!("Comment: {}", comment_text);
    }

    Ok(())
}
