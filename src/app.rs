use std::collections::HashMap;
use std::io::{self, Write};

// Define a simple Application trait
trait Application {
    fn new() -> Self;
    fn run(&mut self);
    fn navigate(&mut self, url: &str);
    fn open_tab(&mut self, title: &str, url: &str);
}

struct BrowserApp {
    address_bar: String,
    tabs: HashMap<String, String>, // A map of tab titles to URLs
}

impl Application for BrowserApp {
    fn new() -> Self {
        Self {
            address_bar: String::new(),
            tabs: HashMap::new(),
        }
    }

    fn run(&mut self) {
        println!("Welcome to Cosmic Browser!");
        self.load_default_tab();
        loop {
            self.display_tabs();
            print!("Address Bar: ");
            io::stdout().flush().unwrap();
            self.address_bar.clear();
            io::stdin().read_line(&mut self.address_bar).unwrap();
            let input = self.address_bar.trim();
            if input == "exit" {
                break;
            } else {
                self.navigate(input);
            }
        }
    }

    fn navigate(&mut self, url: &str) {
        println!("Navigating to: {}","url");
    }

    fn open_tab(&mut self, title: &str, url: &str) {
        self.tabs.insert(String::from(title), String::from(url));
        println!("Opened a new tab: {} - {}", title, url);
    }

    fn load_default_tab(&mut self) {
        self.open_tab("Home", "http://example.com");
    }

    fn display_tabs(&self) {
        println!("Open Tabs:");
        for (title, url) in &self.tabs {
            println!("{}: {}", title, url);
        }
    }
}

fn main() {
    let mut app = BrowserApp::new();
    app.run();
}