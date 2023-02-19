#![allow(
    non_upper_case_globals,
    non_snake_case,
    non_camel_case_types,
)]

#[macro_use]
extern crate bitflags;

use std::env;
use std::fs;

use simple_error::SimpleError;

pub mod utils;
pub mod ese;
pub mod sqlite;
pub mod report;
pub mod fields;

use crate::ese::*;
use crate::sqlite::*;
use crate::report::*;

fn dump(f: &str, format: &ReportFormat) -> Result<(), SimpleError> {
    for entry in fs::read_dir(f).unwrap() {
        if let Ok(e) = entry {
            let p = e.path();
            let metadata = fs::metadata(&p).unwrap();
            if metadata.is_dir() {
                dump(&p.to_string_lossy().into_owned(), format)?;
            } else if let Some(f) = p.file_name() {
                if f == "Windows.edb" {
                    println!("Processing ESE db: {}", p.to_string_lossy());
                    match ese_generate_report(&p, format) {
                        Err(e) => eprintln!("ese_generate_report({}) failed with error: {}", p.to_string_lossy(), e),
                        Ok(()) => {}
                    }
                } else if f == "Windows.db" {
                    println!("Processing sqlite: {}", p.to_string_lossy());
                    match sqlite_generate_report(&p, format) {
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
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("path to dir required");
        return;
    }
    if args[0].contains("help") {
        eprintln!("\nThe Windows Search Forensic Artifact Parser is a RUST based tool designed to parse");
        eprintln!("Windows search artifacts from Windows 10 (and prior) and Windows 11 systems.");
        eprintln!("The tool handle both ESE databases (Windows.edb) and SQLite databases (Windows.db)");
        eprintln!("as input and generate four detailed reports as output.\n");
        eprintln!("[/f format] input\n");
        eprintln!("format: json (default) or csv");
        eprintln!("input: path to dir (which will recursively scan for Windows.edb and Windows.db/Windows-gather.db)");
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
    let dir = args.concat();
    dump(&dir, &format).unwrap();
}
