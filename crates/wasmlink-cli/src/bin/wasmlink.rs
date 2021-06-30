use structopt::StructOpt;
use wasmlink_cli::App;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_module_path(false)
        .init();

    if let Err(e) = App::from_args().execute() {
        log::error!("{:?}", e);
        std::process::exit(1);
    }
}
