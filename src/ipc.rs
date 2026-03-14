use tokio::sync::mpsc;

/// Events the WebView sends back to the iced shell.
#[derive(Debug, Clone)]
pub enum WebViewEvent {
    TitleChanged(String),
    UrlChanged(String),
    FaviconUrl(String),
    LoadStarted,
    LoadFinished,
    LoadProgress(f64), // 0.0 – 1.0
}

/// A cloneable sender the WebView callbacks hold.
pub type EventSender = mpsc::UnboundedSender<WebViewEvent>;
/// The receiver the iced subscription drains.
pub type EventReceiver = mpsc::UnboundedReceiver<WebViewEvent>;

pub fn channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
