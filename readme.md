# HN reply notifier

## Description

A simple daemon, written in rust that watches for replies to your HN comments,
and plays a sound and shows it to you when there is one.

## Getting Started

### Dependencies
* Rust
* An HN account

### Installing
* clone https://github.com/fragmede/hn_reply_notifier
* update `src/main.rsw with your username instead of mine.

### Executing program
* `cargo run`

## Bugs
* Currently, this only checks the first page of comments. To have it watch more pages,
patches welcome!
* No idea if the sleep is the best way to do that in rust. I'm not a rustation so any advice is welcome!

## Authors

[fragmede](github.com/fragmede)

## Version History

* 0.1
    * Initial Release

## License

This project is licensed under the MIT License - see the LICENSE.md file for details

## Acknowledgments

* HN user [`emporas`](https://news.ycombinator.com/user?id=emporas), who, [with this post](https://news.ycombinator.com/item?id=37694270), successfully nerdsniped me
into seeing if ChatGPT could help me write this.

* OpenAI and ChatGPT. I've barely used rust before, and this took me 2 hours to
bang out without knowing what I'm doing.

  * [Transcript of development](https://chat.openai.com/share/646e8440-f566-45ea-9c31-2c43d88b8ab0)

* Google & Stack Overflow. ChatGPT hallucinated the API for rodio, so I had to go old school and [look it up myself](https://stackoverflow.com/questions/74022642/why-cant-rodio-find-my-default-output-device).

