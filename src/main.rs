mod app;
mod browser;
mod ipc;
mod sidebar;
mod toolbar;
mod webview_widget;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("cosmic_browser=debug")
        .init();

    app::run();
}
