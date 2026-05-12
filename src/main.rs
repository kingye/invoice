mod archive;
mod attachment;
mod cli;
mod closing;
mod db;
mod extract_ofd;
mod extract_pdf;
mod extract_xml;
mod import;
mod models;
mod ocr;
mod report;

use clap::Parser;

fn main() {
    let args = cli::Cli::parse();
    if let Err(e) = cli::run(args) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
