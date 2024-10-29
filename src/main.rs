#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types)]

extern crate bitflags;

use clap::Parser;

use std::fs;
use std::io::Write;
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

fn dump(
    f: &str,
    report_prod: &ReportProducer,
    status_logger: &mut Box<dyn Write>,
) -> Result<(), SimpleError> {
    let mut processed = 0;
    match fs::read_dir(f) {
        Ok(dir) => {
            for entry in dir.flatten() {
                let p = entry.path();
                let metadata = fs::metadata(&p).unwrap();
                if metadata.is_dir() {
                    dump(&p.to_string_lossy(), report_prod, status_logger)?;
                } else if is_valid_file(&p) {
                    processed += 1;
                    let _ = match p.extension().and_then(|e| e.to_str()) {
                        Some("edb") => ese_generate_report(&p, report_prod, status_logger),
                        Some("db") => sqlite_generate_report(&p, report_prod, status_logger),
                        _ => continue,
                    };
                }
            }
        }
        Err(e) => panic!("Could not read dir '{f}': {e}"),
    }
    if processed > 0 {
        writeln!(
            status_logger,
            "\nFound {} Windows Search database(s)",
            &processed.to_string()
        )
        .map_err(|e| SimpleError::new(format!("{e}")))
        .unwrap();
    }

    Ok(())
}

fn is_valid_file(p: &PathBuf) -> bool {
    let is_valid_name = p.file_stem()
        .and_then(|s| s.to_str())
        .map_or(false, |name| name.eq_ignore_ascii_case("windows") || name.starts_with("s-1-"));
    let is_valid_ext = p.extension()
        .and_then(|e| e.to_str())
        .map_or(false, |ext| ext == "edb" || ext == "db");
    is_valid_name && is_valid_ext
}


/// Copyright 2023, Aon
///
/// Created by the Stroz Friedberg digital forensics practice at Aon
///
/// SIDR (Search Index DB Reporter) is a Rust-based tool designed to parse Windows search artifacts from Windows 10 (and prior) and Windows 11 systems.
/// The tool handles both ESE databases (Windows.edb) and SQLite databases (Windows.db) as input and generates three detailed reports as output.
///
/// For example, running this command:
///
/// sidr -f json C:\test
///
/// will scan the C:\test directory for Windows.db and Windows.edb files and will produce 3 logs in the current working directory:
///
/// DESKTOP-12345_File_Report_20230307_015244.json
///
/// DESKTOP-12345_Internet_History_Report_20230307_015317.json
///
/// DESKTOP-12345_Activity_History_Report_20230307_015317.json
///
/// Where the filename follows this format:
/// HOSTNAME_ReportName_DateTime.json|csv.
///
/// HOSTNAME is extracted from the database.

#[derive(Parser)]
#[command(author, version, about, long_about)]
struct Cli {
    /// Path to input directory (which will be recursively scanned for Windows.edb and Windows.db).
    input: String,

    /// Output report format
    #[arg(short, long, value_enum, default_value_t = ReportFormat::Json)]
    format: ReportFormat,

    /// Output results to file or stdout
    #[arg(short, long, value_enum, default_value_t = ReportOutput::ToFile)]
    report_type: ReportOutput,

    /// Path to the directory where reports will be created (will be created if not present). Default is the current directory.
    #[arg(short, long, value_name = "OUTPUT DIRECTORY")]
    outdir: Option<PathBuf>,
}

fn main() -> Result<(), SimpleError> {
    let cli = Cli::parse();

    let rep_dir = match cli.outdir {
        Some(outdir) => outdir,
        None => std::env::current_dir().map_err(|e| SimpleError::new(format!("{e}")))?,
    };
    let rep_producer = ReportProducer::new(rep_dir.as_path(), cli.format, cli.report_type);

    let mut status_logger: Box<dyn std::io::Write> = match cli.report_type {
        ReportOutput::ToStdout => Box::new(std::io::sink()),
        ReportOutput::ToFile => Box::new(std::io::stdout()),
    };

    dump(&cli.input, &rep_producer, &mut status_logger)?;
    Ok(())
}
