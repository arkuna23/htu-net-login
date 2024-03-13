use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "htu-net-login")]
#[command(version = "0.1.0")]
#[command(about = "A simple program to manage school net connection", long_about = None)]
struct Args {
    #[cfg(all(feature = "daemon", feature = "tui"))]
    /// Run daemon server
    #[arg(short, long)]
    pub daemon: bool,
}

#[tokio::main]
async fn main() {
    let _app_args = Args::parse();
    #[cfg(all(feature = "daemon", feature = "tui"))]
    if _app_args.daemon {
        daemon::start().await;
    } else {
        tui::run().await.unwrap();
    }
    #[cfg(all(feature = "daemon", not(feature = "tui")))]
    daemon::start().await;
    #[cfg(all(not(feature = "daemon"), feature = "tui"))]
    tui::run().await.unwrap();
}
