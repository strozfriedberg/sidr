#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types)]

extern crate bitflags;
extern crate capture_stdio;

use capture_stdio::OutputCapture;
use crate::capture_stdio::Capture;

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


fn dump(f: &str, report_prod: &ReportProducer, startup_logger: &mut Box<dyn Write + 'static>) -> Result<(), SimpleError> {
    let mut processed = 0;
    match fs::read_dir(f) {
        Ok(dir) => {
            for entry in dir.flatten() {
                let p = entry.path();
                let metadata = fs::metadata(&p).unwrap();
                if metadata.is_dir() {
                    dump(&p.to_string_lossy(), report_prod, startup_logger)?;
                } else if let Some(f) = p.file_name() {
                    if f == "Windows.edb" {
                        writeln!(startup_logger, "Processing ESE db: {}", &p.to_string_lossy()).map_err(|e| SimpleError::new(format!("{e}")))?;
                        if let Err(e) = ese_generate_report(&p, report_prod) {
                            eprintln!(
                                "ese_generate_report({}) failed with error: {}",
                                p.to_string_lossy(),
                                e
                            );
                        }
                        processed += 1;
                    } else if f == "Windows.db" {
                        writeln!(startup_logger, "Processing ESE db: {}", &p.to_string_lossy()).map_err(|e| SimpleError::new(format!("{e}")))?;
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
        }
        Err(e) => panic!("Could not read dir '{f}': {e}"),
    }

    if processed > 0 {
        writeln!(startup_logger, "\nFound {} Windows Search database(s)", &processed.to_string()).map_err(|e| SimpleError::new(format!("{e}")));
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

    dump(&cli.input, &rep_producer, &cli.report_type)?;
    Ok(())
}


#[cfg(test)]
fn capture_stdout(mut output_capture: OutputCapture) -> String {
    use capture_stdio::OutputCapture;
    use crate::capture_stdio::Capture;

    output_capture.restore();
    String::from_utf8(output_capture.get_output().lock().unwrap().clone()).unwrap()
}

#[test]
fn test_print_to_file_only() {
    let ro = ReportOutput::ToFile;

    let output_capture = OutputCapture::capture().unwrap();
    print_to_file_only(format!("gray dog running on the green {}", "grass"), &ro);
    let captured = capture_stdout(output_capture);

    assert_eq!(captured, "gray dog running on the green grass\n");

    let output_capture = OutputCapture::capture().unwrap();
    let dir = fs::read_dir("./tests").unwrap();
    for entry in dir.flatten() {
        let p = entry.path();
        print_to_file_only(format!("test path is {}", &p.to_string_lossy()), &ro);
        let captured = capture_stdout(output_capture);
        assert_eq!(captured, "test path is ./tests/testdata\n");
        break // making sure only tests/testdata is used
    }

    let output_capture = OutputCapture::capture().unwrap();
    let processed = 5;  // reproducing scenario as close as possible to the real code
    print_to_file_only(format!("\nFound {} Windows Search database(s)", &processed.to_string()), &ro);
    let captured = capture_stdout(output_capture);
    assert_eq!(captured, "\nFound 5 Windows Search database(s)\n");
}
