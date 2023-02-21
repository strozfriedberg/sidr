#![allow(
    non_upper_case_globals,
    non_snake_case,
    non_camel_case_types,
)]

#[macro_use]
extern crate bitflags;

use std::env;
use std::fs;
use std::path::PathBuf;

use simple_error::SimpleError;

pub mod utils;
pub mod ese;
pub mod sqlite;
pub mod report;
pub mod fields;

use crate::ese::*;
use crate::sqlite::*;
use crate::report::*;

fn dump(f: &str, report_prod: &ReportProducer) -> Result<(), SimpleError> {
    for entry in fs::read_dir(f).unwrap() {
        if let Ok(e) = entry {
            let p = e.path();
            let metadata = fs::metadata(&p).unwrap();
            if metadata.is_dir() {
                dump(&p.to_string_lossy().into_owned(), report_prod)?;
            } else if let Some(f) = p.file_name() {
                if f == "Windows.edb" {
                    println!("Processing ESE db: {}", p.to_string_lossy());
                    match ese_generate_report(&p, report_prod) {
                        Err(e) => eprintln!("ese_generate_report({}) failed with error: {}", p.to_string_lossy(), e),
                        Ok(()) => {}
                    }
                } else if f == "Windows.db" {
                    println!("Processing sqlite: {}", p.to_string_lossy());
                    match sqlite_generate_report(&p, report_prod) {
                        Err(e) => eprintln!("sqlite_generate_report({}) failed with error: {}", p.to_string_lossy(), e),
                        Ok(()) => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() {
    let mut rep_dir = std::env::current_dir().unwrap();
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("path to directory is required, example of usage:");
        eprintln!("{}", format!("> {} /f json C:\\test", std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy()));
        eprintln!("");
        eprintln!("type /help for more details");
        eprintln!("");
        return;
    }
    if args[0].contains("help") {
        eprintln!("\nThe Windows Search Forensic Artifact Parser is a RUST based tool designed to parse");
        eprintln!("Windows search artifacts from Windows 10 (and prior) and Windows 11 systems.");
        eprintln!("The tool handles both ESE databases (Windows.edb) and SQLite databases (Windows.db)");
        eprintln!("as input and generates four detailed reports as output.\n");
        eprintln!("[/f format] [/outdir directory] input\n");
        eprintln!("format: json (default) or csv.");
        eprintln!("outdir: Path to the directory where reports will be created (will be created if not present).");
        eprintln!("        Default is the current directory.");
        eprintln!(" input: Path to input directory (which will recursively scanned for Windows.edb and Windows.db).");
        eprintln!("");
        eprintln!("Example:");
        eprintln!("{}", format!("> {} /f json C:\\test", std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy()));
        eprintln!("will scan C:\\test directory for Windows.db/Windows.edb files and produce 3 logs:");
        eprintln!("Windows.db/edb.file-report.json");
        eprintln!("Windows.db/edb.ie-report.json");
        eprintln!("Windows.db/edb.act-report.json");
        eprintln!("");
        std::process::exit(0);
    }
    let mut format = ReportFormat::Json;
    if args[0].to_lowercase() == "/f" {
        if args[1].to_lowercase() == "json" {
            format = ReportFormat::Json;
        } else if args[1].to_lowercase() == "csv" {
            format = ReportFormat::Csv;
        } else {
            eprintln!("unknown format: {}", args[1]);
            std::process::exit(-1);
        }
        args.drain(..2);
    }
    if args[0].to_lowercase() == "/outdir" {
        rep_dir = PathBuf::from(args[1].clone());
        args.drain(..2);
    }
    let rep_producer = ReportProducer::new(rep_dir.as_path(), format);
    let dir = args.concat();
    dump(&dir, &rep_producer).unwrap();
}
