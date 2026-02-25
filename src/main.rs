use clap::Parser;
#[cfg(target_os = "windows")]
use rpos::module::Module;
#[cfg(not(target_os = "windows"))]
use rpos::server_client::{server_init, Client};
#[cfg(target_os = "linux")]
mod adc;
mod messages;
mod calibrate;
mod mixer;
#[cfg(target_os = "linux")]
mod elrs_tx;
#[cfg(all(target_os = "linux", feature = "joydev_input"))]
mod joy_dev;
mod joysticks_test;
#[cfg(target_os = "linux")]
mod gampad;
mod crsf_rc_in;
mod stm32_serial;
mod mock_joystick;
#[cfg(target_os = "linux")]
mod usb_gamepad;
mod ui;
mod ui_demo;
mod system_state_mock;

pub const CALIBRATE_FILENAME: &str = "joystick.toml";

#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help(true))]
struct Cli {
    #[arg(long)]
    server: bool,

    /// commands send by clients.
    #[arg(value_name = "client commands")]
    other: Option<Vec<String>>,
}

pub fn client_process_args<T:clap::Parser>(
    argc: u32,
    argv: *const &str
) -> Option<T> {

    let argv = unsafe { std::slice::from_raw_parts(argv, argc as usize) };

    let ret = T::try_parse_from(argv);

    if ret.is_err() {
        let help_str = T::command().render_help();
        rpos::thread_logln!("{}", help_str);
        return None
    }
    ret.ok()
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        run_windows_local_mode();
        return;
    }

    #[cfg(not(target_os = "windows"))]
    run_unix_client_server();
}

#[cfg(not(target_os = "windows"))]
fn run_unix_client_server() {
    const SOCKET_PATH: &str = "./rpsocket";
    let cli = Cli::parse();

    if cli.server {
        let hello_txt = r"
        __    _     ______    
       / /   (_)___/_  __/  __
      / /   / / __ \/ / | |/_/
     / /___/ / / / / / _>  <  
    /_____/_/_/ /_/_/ /_/|_|  ";

        println!("{hello_txt}");

        println!("Built from branch={} commit={} dirty={} source_timestamp={}",
            env!("GIT_BRANCH"),
            env!("GIT_COMMIT"),
            env!("GIT_DIRTY"),
            env!("SOURCE_TIMESTAMP"),
        );

        server_init(SOCKET_PATH).unwrap();
    } else {
        let mut client = Client::new(SOCKET_PATH).unwrap();
        client.send_str(cli.other.unwrap().join(" ").as_str());
        client.block_read();
    }
}

#[cfg(target_os = "windows")]
fn run_windows_local_mode() {
    let cli = Cli::parse();
    if cli.server {
        println!("Windows currently supports local mode only. Run: LinTx -- <module args>");
        return;
    }

    if let Some(other) = cli.other {
        execute_module_inline(&other);
    }
}

#[cfg(target_os = "windows")]
fn execute_module_inline(args: &[String]) {
    if args.is_empty() {
        return;
    }

    let argv: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
    Module::get_module(argv[0]).execute(argv.len() as u32, argv.as_ptr());
}
