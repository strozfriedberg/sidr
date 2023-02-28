#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types)]

#[macro_use]
extern crate bitflags;

use clap::Parser;

use std::fs;
use std::path::PathBuf;

use simple_error::SimpleError;

pub mod ese;
pub mod report;
pub mod shared;
pub mod sqlite;
pub mod utils;

use crate::ese::*;
use crate::report::*;
use crate::sqlite::*;

fn dump(f: &str, report_prod: &ReportProducer) -> Result<(), SimpleError> {
    let mut processed = 0;
    for entry in fs::read_dir(f).unwrap().flatten() {
        let p = entry.path();
        let metadata = fs::metadata(&p).unwrap();
        if metadata.is_dir() {
            dump(&p.to_string_lossy(), report_prod)?;
        } else if let Some(f) = p.file_name() {
            if f == "Windows.edb" {
                println!("Processing ESE db: {}", p.to_string_lossy());
                if let Err(e) = ese_generate_report(&p, report_prod) {
                    eprintln!(
                        "ese_generate_report({}) failed with error: {}",
                        p.to_string_lossy(),
                        e
                    );
                }
                processed += 1;
            } else if f == "Windows.db" {
                println!("Processing sqlite: {}", p.to_string_lossy());
                if let Err(e) = sqlite_generate_report(&p, report_prod) {
                    eprintln!(
                        "sqlite_generate_report({}) failed with error: {}",
                        p.to_string_lossy(),
                        e
                    );
                }
                processed += 1;
            }
        }
    }
    println!("Processed {} Windows Search database(s)", processed);

    Ok(())
}

/// The Windows Search Forensic Artifact Parser is a RUST based tool designed to parse Windows search artifacts from Windows 10 (and prior) and Windows 11 systems.
/// The tool handles both ESE databases (Windows.edb) and SQLite databases (Windows.db) as input and generates four detailed reports as output.
///
/// Example:
/// `> windows_search_artifact -f json C:\\test`
///
/// will scan C:\\test directory for Windows.db/Windows.edb files and produce 3 logs:
///
///  `Windows.db/edb.file-report.json`
///  `Windows.db/edb.ie-report.json`
///  `Windows.db/edb.act-report.json`
#[derive(Parser)]
#[command(author, version, about, long_about)]
struct Cli {
    /// Path to input directory (which will be recursively scanned for Windows.edb and Windows.db).
    input: String,

    /// Output format: json (default) or csv
    #[arg(short, long, value_name = "json")]
    format: Option<ReportFormat>,

    /// Path to the directory where reports will be created (will be created if not present). Default is the current directory.
    #[arg(short, long, value_name = "CURRENT DIR")]
    outdir: Option<PathBuf>,
}

fn main() -> Result<(), SimpleError> {
    let cli = Cli::try_parse().map_err(|e| {
        eprintln!("{}", e.to_string());
        SimpleError::new("Invalid usage of arguments, type --help for a detailed description.")
    })?;

    let format = cli.format.unwrap_or(ReportFormat::Json);
    let rep_dir = match cli.outdir {
        Some(outdir) => outdir,
        None => std::env::current_dir().map_err(|e| SimpleError::new(format!("{}", e)))?,
    };
    let rep_producer = ReportProducer::new(rep_dir.as_path(), format);
    dump(&cli.input, &rep_producer)?;

    Ok(())
}
