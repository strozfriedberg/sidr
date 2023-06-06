#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types)]

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
    match fs::read_dir(f) {
        Ok(dir) =>  {
            for entry in dir.flatten() {
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
        },
        Err(e) => panic!("Could not read dir '{f}': {e}"),
    }

    if processed > 0 {
        println!("Found {} Windows Search database(s)", processed);
    }

    Ok(())
}

/// Copyright 2023, Aon
///
/// Created by the Stroz Friedberg digital forensics practice at Aon
///
/// SIDR (Search Index DB Reporter) is a Rust-based tool designed to parse Windows search artifacts from Windows 10 (and prior) and Windows 11 systems.
/// The tool handles both ESE databases (Windows.edb) and SQLite databases (Windows.db) as input and generates three detailed reports as output.
///
/// Example:
/// `> sidr -f json C:\test`
///
/// will scan C:\test directory for Windows.db/Windows.edb files and produce 3 logs:
///
/// `DESKTOP-POG7R45_File_Report_20230307_015244.json`
/// `DESKTOP-POG7R45_Internet_History_Report_20230307_015317.json`
/// `DESKTOP-POG7R45_Activity_History_Report_20230307_015317.json`
///
/// Where the log name consists of:
/// `HOSTNAME_ReportName_DateTime.json|csv`
///
/// `HOSTNAME` is extracted from the database

#[derive(Parser)]
#[command(author, version, about, long_about)]
struct Cli {
    /// Path to input directory (which will be recursively scanned for Windows.edb and Windows.db).
    input: String,

    /// Output format: json (default) or csv
    #[arg(short, long, value_enum, default_value_t = ReportFormat::Json)]
    format: ReportFormat,

    /// Report Type: ToFile or ToStdout
    #[arg(short, long, value_enum, default_value_t = ReportType::ToFile)]
    report_type: ReportType,

    /// Path to the directory where reports will be created (will be created if not present). Default is the current directory.
    #[arg(short, long, value_name = "OUTPUT DIRECTORY")]
    outdir: Option<PathBuf>,
}

fn main() -> Result<(), SimpleError> {
    let cli = Cli::parse();

    let rep_dir = match cli.outdir {
        Some(outdir) => outdir,
        None => std::env::current_dir().map_err(|e| SimpleError::new(format!("{}", e)))?,
    };
    let rep_producer = ReportProducer::new(rep_dir.as_path(), cli.format, cli.report_type);

    dump(&cli.input, &rep_producer)?;
    Ok(())
}
