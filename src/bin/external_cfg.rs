use clap::Parser;
use std::path::PathBuf;
use walkdir::WalkDir;
use env_logger::{self, Target};
use serde_yaml;
use wsa_lib::{ReportsCfg, do_reports};
use wsa_lib::report::ReportFormat;

#[derive(Parser)]
struct Cli {
    /// Path to <config.yaml>
    #[arg(short, long)]
    cfg_path: String,

    /// Path to input directory (which will be recursively scanned for Windows.edb and Windows.db).
    input: String,

    /// Output format: json (default) or csv
    #[arg(short, long, value_enum, default_value_t = ReportFormat::Json)]
    format: ReportFormat,

    /// Path to the directory where reports will be created (will be created if not present). Default is the current directory.
    #[arg(short, long, value_name = "OUTPUT DIRECTORY")]
    outdir: Option<PathBuf>,
}

fn do_sql_report(db_path: &str, cfg: &ReportsCfg) {
    let mut sql_reader = wsa_lib::SqlReader::new_(db_path);
    do_reports(cfg, &mut sql_reader);
}

fn do_edb_report(db_path: &str, cfg: &ReportsCfg) {
    let mut edb_reader = wsa_lib::EseReader::new(db_path, &cfg.table_edb);
    do_reports(cfg, &mut edb_reader);
}

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .target(Target::Stderr)
        .init();

    // for s in env::args() {
    //     println!("{s}");
    // }
    let cli = Cli::parse();
    let s = std::fs::read_to_string(&cli.cfg_path).unwrap();
    let mut cfg: ReportsCfg = serde_yaml::from_str(s.as_str()).unwrap();

    if let Some(output_dir) = &cli.outdir {
        cfg.output_dir = output_dir.to_str().unwrap().to_string();
    }

    cfg.output_format = match cli.format {
        ReportFormat::Json => wsa_lib::OutputFormat::Json,
        ReportFormat::Csv => wsa_lib::OutputFormat::Csv,
    };

    static DB_NAMES: [&'static str; 2] = ["Windows.edb", "Windows.db"];

    for entry in WalkDir::new(&cli.input).into_iter().filter_entry(|e| e.file_type().is_dir() || DB_NAMES.contains(&e.file_name().to_str().unwrap())) {
        if let Ok(ref e) = entry {
            if !e.file_type().is_dir() {
                let db_path = e.path().to_str().unwrap().to_string();

                println!("{db_path}");
                if db_path.ends_with(".edb") {
                    do_edb_report(&db_path, &cfg);
                } else if db_path.ends_with(".db") {
                    do_sql_report(&db_path, &cfg);
                }
            }
        }
    }

}
