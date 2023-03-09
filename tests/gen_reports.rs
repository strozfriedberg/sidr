#[cfg(test)]
//use wsa_lib::*;
use std::fs;
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use tempdir::TempDir;

fn press_any() {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    write!(stdout, "Press any key to continue...").unwrap();
    stdout.flush().unwrap();
    use std::io::prelude::*;
    let _ = stdin.read(&mut [0u8]).unwrap();
}

#[test]
fn compare_with_sql_select() {
    let reporter_bin = "external_cfg";
    let reporter_bin_path = format!("target/debug/{reporter_bin}");
    let env_db_path = "WSA_TEST_WINDOWS_DB_PATH";
    let env_cfg_path = "WSA_TEST_CONFIGURATION_PATH";
    let env_export_csv_path = "WSA_TEST_EXPORT_CSV_PATH";
    let db_path = env::var(env_db_path).expect(format!("Error getting environment variable '{env_db_path}'").as_str());
    let cfg_path = env::var(env_cfg_path).expect(format!("Error getting environment variable '{env_cfg_path}'").as_str());
    let export_csv_path = env::var(env_export_csv_path).expect(format!("Error getting environment variable '{env_export_csv_path}'").as_str());
    let work_dir_name = format!("{reporter_bin}_testing");
    let work_dir = TempDir::new(work_dir_name.as_str()).expect("{work_dir_name} creation");

    println!("db_path: {db_path}");
    println!("cfg_path: {cfg_path}");
    println!("work_dir: {work_dir:?}");

    let mut cmd = Command::new(reporter_bin_path.as_str());
    let cmd = cmd
        .args(["--db-path", db_path.as_str()])
        .args(["--cfg-path", cfg_path.as_str()])
        .args(["--outdir", work_dir.path().to_str().unwrap()])
        .args(["--format", "csv"]);

    println!("cmd '{cmd:?}'");

    let output =
        // Command::new(reporter_bin_path.as_str())
        // .args(args)
        cmd
        .stderr(Stdio::inherit())
        .spawn()
        .expect(format!("'{cmd:?}' command failed to start").as_str());

    if let Some(stderr) = output.stderr {
        panic!("stderr: {stderr:?}");
    }

    let mut cmd =
        Command::new("sqlite3");
    let mut cmd = cmd
        .current_dir(work_dir.path().to_str().unwrap())
        .arg("-readonly")
        .arg("-echo")
        .args(["-init", export_csv_path.as_str()])
        .arg(db_path)
        ;

    println!("cmd '{cmd:?}'");

    let output =
        // Command::new(reporter_bin_path.as_str())
        // .args(args)
        cmd
            .stderr(Stdio::inherit())
            .spawn()
            .expect(format!("'{cmd:?}' command failed to start").as_str());

    if let Some(stderr) = output.stderr {
        panic!("stderr: {stderr:?}");
    }

    press_any();

    fs::remove_dir_all(work_dir);
}
