use clap::Parser;

use crate::{
    client_process_args,
    ui::{
        app::UiApp,
        backend::{new_backend, BackendKind},
    },
};

#[derive(Parser)]
#[command(name = "ui_demo", about = "LinTX launcher app", long_about = None)]
struct Cli {
    #[arg(long, default_value = "sdl")]
    backend: String,

    #[arg(long, default_value = "/dev/fb0")]
    fb_device: String,

    #[arg(long, default_value_t = 30)]
    fps: u32,

    #[arg(long, default_value_t = 800)]
    width: u32,

    #[arg(long, default_value_t = 480)]
    height: u32,
}

fn ui_demo_main(argc: u32, argv: *const &str) {
    let args = match client_process_args::<Cli>(argc, argv) {
        Some(a) => a,
        None => return,
    };

    let backend_kind = BackendKind::parse(&args.backend, &args.fb_device, args.width, args.height);
    let mut backend = new_backend(backend_kind);

    let mut app = UiApp::new();
    app.run(backend.as_mut(), args.fps);
}

#[rpos::ctor::ctor]
fn register() {
    rpos::module::Module::register("ui_demo", ui_demo_main);
}
