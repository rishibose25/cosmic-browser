use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum WebViewEvent {
    TitleChanged        { tab_id: usize, title: String },
    UrlChanged          { tab_id: usize, url: String },
    FaviconUrl          { tab_id: usize, url: String },
    LoadStarted         { tab_id: usize },
    LoadFinished        { tab_id: usize },
    LoadProgress        { tab_id: usize, progress: f64 },
    CanGoChanged        { tab_id: usize, back: bool, forward: bool },
    IpcMessage          { tab_id: usize, body: String },
    NewWindowRequested  { tab_id: usize, url: String },
    DownloadStarted     { tab_id: usize, url: String, suggested_path: Option<String> },
    PermissionRequested { tab_id: usize, permission: String },
    NavigationBlocked   { tab_id: usize, url: String },
}

pub type EventSender   = mpsc::UnboundedSender<WebViewEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<WebViewEvent>;

pub fn channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
