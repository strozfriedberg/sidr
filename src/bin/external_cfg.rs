use clap::Parser;
use std::path::PathBuf;
use env_logger::{self, Target};
use serde_yaml;
use wsa_lib::{ReportsCfg, do_reports};

#[derive(Parser)]
struct Cli {
    /// Path to <config.yaml>
    #[arg(short, long)]
    cfg_path: String,
    /// Path to the directory where reports will be created (will be created if not present).
    /// Default is the current directory.
    #[arg(short, long)]
    outdir: Option<String>,
    /// json (default) or csv.
    #[arg(short, long)]
    format: Option<String>,
    /// Path to SQL/EDB database
    #[arg(short, long)]
    db_path: PathBuf,
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
    let db_path = cli.db_path.display().to_string();

    if let Some(output_dir) = &cli.outdir {
        cfg.output_dir = output_dir.clone();
    }

    if let Some(output_format) = &cli.format {
        cfg.output_format = match output_format.to_lowercase().as_str() {
            "json" => wsa_lib::OutputFormat::Json,
            "csv" => wsa_lib::OutputFormat::Csv,
            _ => panic!("Unknow output format '{output_format}'"),
        }
    }

    if db_path.ends_with("Windows.edb") {
        do_edb_report(db_path.as_str(), &cfg);
    } else if db_path.ends_with("Windows.db") {
        do_sql_report(db_path.as_str(), &cfg);
    }
}
