//extern crate core;

use std::env;
#[cfg(test)]

const SQL_GEN_SCRIPT: &str = "sql_2_csv.py";
const JSON_TO_CSV: &str = "json_2_csv.py";
const ESE_TO_CSV: &str = "ese_2_csv.py";

use once_cell::sync::OnceCell;
static PYTHON_SCRIPTS_PATH: OnceCell<String> = OnceCell::new();
static WORK_DIR: OnceCell<String> = OnceCell::new();

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use chrono::TimeZone;
use clap::__macro_refs::once_cell;

use env_logger::{self, Target};
use log::info;
use tempdir::TempDir;
use glob::glob;

use ese_parser_lib::esent::ese_api::EseAPI;
use ese_parser_lib::ese_trait::{ESE_MoveFirst, ESE_MoveNext, EseDb};
use ese_parser_lib::vartime::{SYSTEMTIME, VariantTimeToSystemTime};

use wsa_lib::{ColumnType, ReportsCfg, utils};
use wsa_lib::utils::{format_date_time, from_utf16, get_date_time_from_filetime};

fn glob_vec_path(pattern: &str) -> Vec<PathBuf> {
    glob(pattern).unwrap().map(|r| r.unwrap()).collect()
}

fn glob_vec_string(pattern: &str) -> Vec<String> {
    glob_vec_path(pattern).into_iter().map(|p| p.to_str().unwrap().to_string()).collect()
}

fn glob_vec_names(pattern: &str) -> Vec<String> {
    glob_vec_path(pattern).into_iter().map(|p| p.file_name().unwrap().to_str().unwrap().to_string()).collect()
}

fn get_env(var: &str) -> String {
    env::var(var).expect(format!("Error getting environment variable '{var}'").as_str())
}

fn remove_files(pattern: &str) {
    let paths = glob_vec_path(pattern);
    paths
        .iter()
        .for_each(|p| fs::remove_file(p).expect(format!("remove '{}' failed", p.display()).as_str()));
}

fn do_invoke(cmd: &mut Command) {
    let args: Vec<&str> = cmd.get_args().into_iter().map(|a| a.to_str().unwrap()).collect();
    info!("cmd '{} {}'", cmd.get_program().to_str().unwrap(), args.join(" "));
    // if let Some(cur_dir) = cmd.get_current_dir() {
    //     info!("current_dir: {}", cur_dir.display());
    // }

    let mut child = cmd
        .stderr(Stdio::inherit())
        .spawn()
        .expect(format!("'{cmd:?}' command failed to start").as_str());

    if !child.wait().unwrap().success() {
        if let Some(stderr) = child.stderr {
            panic!("stderr: {stderr:?}");
        }
        panic!("Failed '{cmd:?}'");
    }
}

fn invoke_python(script: &str, args: &[&str]) {
    let mut cmd =
        Command::new("python");
    let cmd = cmd
        .current_dir(WORK_DIR.get().unwrap())
        .arg(format!("{}/{script}", PYTHON_SCRIPTS_PATH.get().unwrap()));

    for arg in args {
        cmd.arg(arg);
    }

    do_invoke(cmd);
}


fn generate_csv_json(reporter_bin_path: &str, common_args: &Vec<&str>) {
    info!("generate_csv_json");

    let mut cmd = Command::new(&reporter_bin_path);
    let cmd = cmd
        .args(&*common_args)
        .arg("csv");

    do_invoke(cmd);

    let mut cmd = Command::new(&reporter_bin_path);
    let cmd = cmd
        .args(&*common_args)
        .arg("json");

    do_invoke(cmd);
}

fn generate_reports(reporter_bin_path: &str, db_path: &str, common_args: &Vec<&str>) {
    let mut args = vec!["--db-path", db_path];
    args.extend(common_args);
    generate_csv_json(reporter_bin_path, &args);
}

fn do_generate(reporter_bin_path: &str, db_path: &str, cfg_path: &str) {
    let work_dir = WORK_DIR.get().unwrap();

    remove_files(format!("{}/*.csv", work_dir).as_str());
    remove_files(format!("{}/*.json", work_dir).as_str());

    let common_args = vec!["--cfg-path", cfg_path, "--outdir", work_dir, "--format"];
    generate_reports(&reporter_bin_path, &db_path, &common_args);
}

fn do_sqlite3(db_path: &str, work_dir: &String) {
    let scripts = glob_vec_names(format!("{work_dir}/*.sql").as_str());
    for script in &scripts {
        let mut cmd =
            Command::new("sqlite3");
        let cmd = cmd
            .current_dir(work_dir)
            .arg(db_path)
            .arg(format!(".read {}", script.as_str()))
            ;

        do_invoke(cmd);
    }
}

fn do_sql_test(reporter_bin_path: &str, db_path: &str, cfg_path: &str, sqlite3ext_h_path: &str) {
    info!("do_sql_test");

    do_generate(reporter_bin_path, db_path, cfg_path);

    let work_dir = WORK_DIR.get().unwrap();
    let files_to_copy = ["dtformat.c"];
    for file in files_to_copy {
        fs::copy(format!("tests/{file}"),
                 format!("{work_dir}/{file}"))
            .expect("copy file '{file}'");
    }

    let mut cmd =
        Command::new("clang");
    let cmd = cmd
        .current_dir(work_dir)
        .arg("-shared")
        .args(["-I", sqlite3ext_h_path])
        .arg("-m32")
        .args(["-o", "dtformat.dll"])
        .arg("dtformat.c");

    do_invoke(cmd);

    invoke_python(JSON_TO_CSV, &[]);
    invoke_python(SQL_GEN_SCRIPT, &[&cfg_path]);

    do_sqlite3(&db_path, work_dir)
}

type ColTitle = String;
type ColId = u32;

#[derive(Debug, Clone)]
struct EseCol {
    col_id: ColId,
    col_type: ColumnType,
}

#[derive(Debug, Clone)]
struct ColInfo {
    col_title: ColTitle,
    ese_col: EseCol,
}

type ColInfos = Vec<ColInfo>;

pub fn dt_to_string(v: Vec<u8>) -> String {
    let filetime = u64::from_le_bytes(v.try_into().unwrap());
    let dt = get_date_time_from_filetime(filetime);
    format_date_time(dt)
}

fn do_ese_test(reporter_bin_path: &str, db_path: &str, cfg_path: &str) {
    info!("do_ese_test");

    do_generate(reporter_bin_path, db_path, cfg_path);

    let s = std::fs::read_to_string(cfg_path).unwrap();
    let cfg: ReportsCfg = serde_yaml::from_str(&s).unwrap();
    let tablename = cfg.table_edb;
    let jdb: Box<dyn EseDb> = Box::new(EseAPI::load_from_path(db_path).unwrap());
    let table = jdb.open_table(&tablename).unwrap();
    let ese_cols = jdb.get_columns(&tablename).unwrap();
    let work_dir = WORK_DIR.get().unwrap();

    for report in &cfg.reports {
        let columns = &report.columns;
        let mut col_infos = ColInfos::with_capacity(columns.len());

        for col_pair in columns {
            let name = col_pair.edb.name.clone();
            if !name.is_empty() {
                let col = ese_cols
                    .iter()
                    .find(|c| c.name == name)
                    .expect(format!("could not find {name}").as_str());
                let ese_col = EseCol {
                    col_id: col.id,
                    col_type: col_pair.kind,
                };

                col_infos.push(ColInfo {
                    col_title: col_pair.title.clone(),
                    ese_col: ese_col.clone(),
                });
            }
        }
        info!("{}: {col_infos:?}", report.title);

        let report_path = format!("{work_dir}/{}_ese.csv", report.title.clone().replace(|c| "\\/ ".contains(c), "_"));
        info!("Writing {report_path}");
        let writer = csv::Writer::from_path(&report_path);
        let mut writer = writer.expect(format!("Could not create '{}'", &report_path).as_str());

        for column in &col_infos {
            writer.write_field(column.col_title.as_str()).unwrap();
        }
        writer.write_record(None::<&[u8]>).unwrap();

        if jdb.move_row(table, ESE_MoveFirst).unwrap() {
            loop {
                for column in &col_infos {
                    let col = &column.ese_col;
                    let mut s= "".to_string();
                    //info!("{}: '{}'", column.col_title, column.ese_col.col_name);
                    match col.col_type {
                        ColumnType::String => match jdb.get_column(table, col.col_id) {
                            Ok(r) =>
                                if let Some(v)  = r {
                                    s = from_utf16(v.as_slice());
                                },
                            Err(e) => panic!("Error reading {}: {e}", column.col_title),
                        }
                        ColumnType::DateTime => match jdb.get_column(table, col.col_id) {
                            Ok(r) =>
                                if let Some(v) = r {
                                    if let Ok(val) = v.clone().try_into() {
                                        let vartime = f64::from_le_bytes(val);
                                        let mut st = SYSTEMTIME::default();
                                        if VariantTimeToSystemTime(vartime, &mut st) {
                                            let datetime = chrono::Utc
                                                .with_ymd_and_hms(
                                                    st.wYear as i32,
                                                    st.wMonth as u32,
                                                    st.wDay as u32,
                                                    st.wHour as u32,
                                                    st.wMinute as u32,
                                                    st.wSecond as u32,
                                                )
                                                .single()
                                                .unwrap(); // this is obviously not the right function! I didn't know what the right one was off the top of my head. We need to include the time component. also needs to be something that returns a DateTime.
                                            s = utils::format_date_time(datetime);
                                        } else {
                                            let filetime = u64::from_le_bytes(v.try_into().unwrap());
                                            let datetime = get_date_time_from_filetime(filetime);
                                            s = utils::format_date_time(datetime);
                                        }
                                    }
                                },
                            Err(e) => panic!("Error reading {}: {e}", column.col_title),
                        }
                        ColumnType::Integer => match jdb.get_column(table, col.col_id) {
                            Ok(r) =>
                                if let Some(v) = r {
                                    s = match v.len() {
                                        1 => u8::from_le_bytes(v[..].try_into().unwrap()).to_string(),
                                        2 => u16::from_le_bytes(v[..].try_into().unwrap()).to_string(),
                                        4 => u32::from_le_bytes(v[..].try_into().unwrap()).to_string(),
                                        8 => u64::from_le_bytes(v[..].try_into().unwrap()).to_string(),
                                        _ => panic!("Integer: {v:?}")
                                    };
                                },
                            Err(e) => panic!("Error reading {}: {e}", column.col_title),
                        }
                        _ => panic!("unexpected type")
                    }
                    //print!("{}: {s}, ", column.col_title);
                    writer.write_field(&s).expect(format!("Error writing '{s}' ({})", column.col_title).as_str());
                }

                //println!("");
                writer.write_record(None::<&[u8]>).unwrap();
                if !jdb.move_row(table, ESE_MoveNext).unwrap() {
                    break;
                }
            }
        }
    }

    remove_files(format!("{}/*.sql", work_dir).as_str());

    invoke_python(JSON_TO_CSV, &[]);
    invoke_python(ESE_TO_CSV, &[&cfg_path]);

    do_sqlite3("", work_dir)
}

#[test]
fn compare_with_sql_select() {
    env_logger::builder()
        .target(Target::Stderr)
        .init();

    info!("compare_with_sql_select");

    let reporter_bin = "external_cfg";
    let reporter_bin_path = format!("target/debug/{reporter_bin}");
    let db_path = get_env("WSA_TEST_WINDOWS_DB_PATH");
    let edb_path = get_env("WSA_TEST_WINDOWS_EDB_PATH");
    let cfg_path = get_env("WSA_TEST_CONFIGURATION_PATH");
    let python_scripts_path = get_env("WSA_TEST_PYTHON_SCRIPTS_PATH");
    let sqlite3ext_h_path = get_env("ENV_SQLITE3EXT_H_PATH");
    let work_dir_name = format!("{reporter_bin}_testing");
    let work_temp_dir = TempDir::new(work_dir_name.as_str()).expect("{work_dir_name} creation");
    let _work_dir_keeper;
    let work_dir = if env::var("KEEP_TEMP_WORK_DIR").is_ok() {
        _work_dir_keeper = work_temp_dir.into_path();
        _work_dir_keeper.as_path().to_str().unwrap()
    } else {
        work_temp_dir.path().to_str().unwrap()
    };

    info!("db_path: {db_path}");
    info!("edb_path: {edb_path}");
    info!("cfg_path: {cfg_path}");
    info!("work_dir: {work_dir}");
    info!("python_scripts_path: {python_scripts_path}");

    PYTHON_SCRIPTS_PATH.set(python_scripts_path).expect("PYTHON_SCRIPTS_PATH.set failed");
    WORK_DIR.set(work_dir.to_string()).expect("WORK_DIR.set failed");

    do_sql_test(&reporter_bin_path, &db_path, &cfg_path, &sqlite3ext_h_path);
    do_ese_test(&reporter_bin_path, &edb_path, &cfg_path);

    let diffs = glob_vec_string(format!("{work_dir}/*.discrepancy").as_str());
    let mut failed = Vec::<String>::with_capacity(diffs.len());
    for diff in &diffs {
        let count = fs::read_to_string(diff).expect(format!("error reading {diff}").as_str());
        if count.trim() != "0" {
            failed.push(format!("'{diff}' has {count} discrepancies"));
        }
    }

    if failed.len() != 0 {
        panic!("{}", failed.join("\n"));
    }
}

