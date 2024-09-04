use clap::Parser;
use service::BIN_PATH;
mod service;
mod util;

#[derive(Parser, Debug)]
#[command(name = "htu-net-login")]
#[command(version = "0.1.1")]
#[command(about = "A simple program to manage school network connection", long_about = None)]
struct Args {
    /// Run daemon server
    #[cfg(feature = "daemon")]
    #[arg(short, long)]
    pub daemon: bool,

    /// Run without console(only effective in daemon mode)
    #[cfg(feature = "daemon")]
    #[arg(long, short)]
    pub background: bool,

    /// Run tui
    #[cfg(feature = "tui")]
    #[arg(short, long)]
    pub ui: bool,

    #[cfg(feature = "tui")]
    /// Tui min tick rate
    #[arg(short, long, default_value_t = 20)]
    pub tick_rate: u16,

    #[cfg(feature = "tui")]
    /// Tui frame rate
    #[arg(short, long, default_value_t = 30)]
    pub frame_rate: u16,

    /// Uninstall program
    #[arg(long)]
    pub uninstall_daemon: bool,

    /// Install program
    #[arg(long)]
    pub install: bool,
}

async fn uninstall_daemon(_: &mut Args) {
    #[cfg(windows)]
    {
        if let Err(e) = service::uninstall_daemon().await {
            eprintln!("服务卸载失败: {:?}", e);
        } else {
            println!("服务卸载成功");
        }
    }
}

async fn install(_: &mut Args) {
    #[cfg(windows)]
    {
        if !service::is_admin() {
            runas::Command::new("cmd")
                .args(&[
                    "/C",
                    &format!(
                        "{} --install & pause",
                        std::env::current_exe().unwrap().to_str().unwrap()
                    ),
                ])
                .status()
                .unwrap();
            return;
        } else {
            let _ = service::uninstall_daemon().await;
            let _ = tokio::fs::remove_file(BIN_PATH).await;
            if let Err(e) = service::install_bin().await {
                eprintln!("程序安装失败: {:?}", e);
            } else {
                println!("程序安装成功");
            }

            #[cfg(feature = "daemon")]
            if let Err(e) = service::install_daemon().await {
                eprintln!("服务安装失败: {:?}", e);
                return;
            } else {
                println!("服务安装成功");
            }

            util::windows::create_shortcut(
                BIN_PATH.into(),
                "--ui",
                dirs::desktop_dir()
                    .expect("desktop dir not found")
                    .join("HtuNet.lnk"),
                "河师大校园网自动登录",
            )
            .expect("failed to create shortcut");

            println!("-----------------------------------------------------------------------------------------");
            println!("安装完成，请重启电脑，之后再次打开程序设定账号使自动登录生效");
        }
    }
}

async fn run_tui(app_args: &Args) {
    if let Err(e) = htu_net_login_tui::run(app_args.frame_rate, app_args.tick_rate).await {
        eprintln!("TUI 运行错误: {}", e)
    }
}

#[tokio::main(worker_threads = 2)]
async fn main() {
    let mut app_args = Args::parse();

    // run daemon
    #[cfg(feature = "daemon")]
    if app_args.daemon {
        service::run_service(app_args.background).await.unwrap();
        return;
    };

    #[cfg(feature = "daemon")]
    if app_args.uninstall_daemon {
        if service::is_bin_exists() {
            uninstall_daemon(&mut app_args).await;
        } else {
            eprintln!("程序未安装");
        }
        return;
    };

    #[cfg(feature = "tui")]
    if app_args.ui {
        run_tui(&app_args).await;
        return;
    }

    if app_args.install {
        install(&mut app_args).await;
    } else if !service::is_installed().unwrap() {
        println!("程序未安装，尝试安装...");
        install(&mut app_args).await;
    } else {
        run_tui(&app_args).await;
    }
}
