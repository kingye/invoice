mod cli;
mod db;
mod models;
mod import;
mod extract_xml;
mod extract_pdf;
mod extract_ofd;
mod attachment;
mod report;
mod archive;
mod closing;

use clap::Parser;

fn main() {
    let args = cli::Cli::parse();
    if let Err(e) = cli::run(args) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
