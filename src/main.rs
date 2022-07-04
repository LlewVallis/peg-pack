mod cli;
mod core;
mod loader;
mod output;
mod runtime;
mod store;

fn main() {
    cli::setup_panic_hook();
    cli::run();
}
