mod archive;
mod attachment;
mod cli;
mod closing;
mod db;
mod mcp;
mod extract_ofd;
mod extract_pdf;
mod extract_xml;
mod import;
mod models;
mod ocr;
mod ops;
mod report;

use clap::Parser;

fn main() {
    let args = cli::Cli::parse();
    if let Err(e) = cli::run(args) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
