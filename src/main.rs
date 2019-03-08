#[macro_use]
extern crate clap;
extern crate console;
extern crate headless_chrome;
extern crate indicatif;

use std::error::Error;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

use clap::ArgMatches;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use indicatif::{ProgressBar, ProgressStyle};

static URL: &str = "https://www.vocabulary.com/dictionary";

#[derive(Debug)]
struct SomeError {
  msg: &'static str,
}

impl Error for SomeError {}
impl std::fmt::Display for SomeError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{}", self.msg)
  }
}

fn lookup(word: &str, tx: Sender<()>) -> Result<(), Box<Error>> {
  let browser = Browser::new(LaunchOptionsBuilder::default().build().unwrap())?;
  let tab = browser.wait_for_initial_tab()?;
  tab.navigate_to(&format!("{}/{}", URL, word))?;

  if let Ok(_) = tab.wait_for_element_with_custom_timeout("div.didyoumean", 5_000) {
    return Err(Box::new(SomeError {
      msg: "Definition Not found.",
    }));
  }

  let p = tab.wait_for_element_with_custom_timeout("p.short", 5_000);
  let h3 = tab.wait_for_element_with_custom_timeout("h3.definition", 5_000);
  let _ = tx.send(());

  if let Ok(short) = p {
    if let Ok(s) = short.call_js_fn("function () { return this.textContent; }") {
      let mut i = 0;
      println!(
        "{}",
        console::style(s.value.unwrap().as_str().unwrap().split(' ').fold(
          String::new(),
          |mut acc, w| {
            if i % 15 == 0 {
              acc.push('\n');
            } else {
              acc.push(' ');
            }
            for c in w.chars() {
              acc.push(c);
            }
            i += 1;
            acc
          }
        ))
        .italic()
        .bold()
        .yellow(),
      )
    }
  }
  if let Ok(definition) = h3 {
    if let Ok(d) = definition.call_js_fn("function () { return this.innerText; }") {
      println!(
        "\n  {}",
        console::style(d.value.unwrap().as_str().unwrap().replace("\n", ". - "))
          .italic()
          .cyan(),
      );
    }
  }

  Ok(())
}

fn main() {
  let args: ArgMatches = clap_app!(app =>
    (version: "0.1.0")
    (author: "Martin <mycha0@hotmail.com>")
    (about: "Lookup word definition from vocabulary.com")
    (@arg WORD: +required +takes_value "The word to look up")
  )
  .get_matches_safe()
  .unwrap_or_else(|e| e.exit());

  let word = args.value_of("WORD").unwrap();

  let (tx, rx) = channel();
  thread::spawn(move || {
    let spinner = ProgressBar::new_spinner();
    spinner.set_message(&format!(
      "{} {}",
      console::style("Retrieving definition from").bold(),
      console::style(URL).underlined(),
    ));
    spinner.set_style(ProgressStyle::default_spinner());
    loop {
      if rx.try_recv().is_ok() {
        break;
      }
      spinner.inc(1);
      thread::sleep(Duration::from_millis(100));
    }
    spinner.finish();
  });

  let _ = lookup(word, tx).map_err(|e| eprintln!("Error: {}", e));
}
