use std::env;
#[cfg(test)]
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use env_logger::{self, Target};
use log::info;
use tempdir::TempDir;
use glob::glob;

use libesedb::{self, EseDb};

use wsa_lib::ReportsCfg;
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

fn do_generate(reporter_bin_path: &str, db_path: &str, cfg_path: &str, work_dir: &str) {
    remove_files(format!("{}/*.csv", work_dir).as_str());
    remove_files(format!("{}/*.json", work_dir).as_str());

    let common_args = vec!["--cfg-path", cfg_path, "--outdir", work_dir, "--format"];
    generate_reports(&reporter_bin_path, &db_path, &common_args);
}

fn do_sql_test(reporter_bin_path: &str, db_path: &str, cfg_path: &str, sql_generator_path: &str, sql_to_csv_path: &str, work_dir: &str) {
    info!("do_sql_test");

    do_generate(reporter_bin_path, db_path, cfg_path, work_dir);

    let mut cmd =
        Command::new("python");
    let cmd = cmd
        .current_dir(work_dir)
        .arg(sql_to_csv_path);

    do_invoke(cmd);

    let mut cmd =
        Command::new("python");
    let cmd = cmd
        .current_dir(work_dir)
        .arg(sql_generator_path)
        .arg(cfg_path);

    do_invoke(cmd);

    let scripts = glob_vec_names(format!("{work_dir}/*.sql").as_str());
    for script in &scripts {
        let mut cmd =
            Command::new("sqlite3");
        let cmd = cmd
            .current_dir(work_dir)
            .arg(&db_path)
            .arg(format!(".read {}", script.as_str()))
            ;

        do_invoke(cmd);
    }

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

type ColTitle = String;
type ColName = String;
type ColInd = usize;

#[derive(Debug, Clone)]
struct EseCol {
    col_name: ColName,
    col_ind: ColInd,
}

#[derive(Debug)]
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

fn do_ese_test(reporter_bin_path: &str, db_path: &str, cfg_path: &str, work_dir: &str) {
    info!("do_ese_test");

    do_generate(reporter_bin_path, db_path, cfg_path, work_dir);

    let s = std::fs::read_to_string(cfg_path).unwrap();
    let cfg: ReportsCfg = serde_yaml::from_str(&s).unwrap();
    let tablename = cfg.table_edb;
    let db = EseDb::open(db_path).unwrap();
    let table = db.table_by_name(tablename.as_str()).unwrap();
    let ese_cols: Vec<EseCol> = table
        .iter_columns()
        .unwrap()
        .enumerate()
        .map(|(i, c)| {
            let col = c.as_ref().unwrap();
            EseCol {
                col_name: col.name().unwrap().clone(),
                col_ind: i,
            }
        })
        .collect();

    for report in &cfg.reports {
        let columns = &report.columns;
        let mut col_infos = ColInfos::with_capacity(columns.len());

        for col_pair in columns {
            let name = col_pair.edb.name.clone();
            if !name.is_empty() {
                let col = ese_cols
                    .iter()
                    .find(|c| *c.col_name == name)
                    .expect(format!("could not find {name}").as_str());
                col_infos.push(
                    ColInfo {
                        col_title: col_pair.title.clone(),
                        ese_col: col.clone(),
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

        for row in table.iter_records().unwrap() {
            for column in &col_infos {
                //info!("{}: '{}'", column.col_title, column.ese_col.col_name);
                let val = row.as_ref().unwrap().value(column.ese_col.col_ind as i32).unwrap();
                match val {
                    libesedb::Value::Null(()) =>
                        writer.write_field("").expect("Error writing Null"),
                    libesedb::Value::Binary(x) =>
                        if column.col_title=="System_Size" {
                            let v = u64::from_le_bytes(x.try_into().unwrap());
                            let s = if v==3038287259199220266_u64 {
                                "".to_string()
                            } else {
                                format!("{v}")
                            };
                            writer.write_field(&s).expect(format!("Error writing u64 '{s}' (from Binary)").as_str())
                        } else {
                            writer.write_field(dt_to_string(x)).expect("Error writing DateTime (from Binary)")
                        }
                    libesedb::Value::Text(s) =>
                        writer.write_field(&s).expect(format!("Error writing Text '{s}'").as_str()),
                    libesedb::Value::I32(x) =>
                        writer.write_field(format!("{x}")).expect(format!("Error writing I32 '{x}'").as_str()),
                    libesedb::Value::U32(x) =>
                        writer.write_field(format!("{x}")).expect(format!("Error writing U32 '{x}'").as_str()),
                    libesedb::Value::LargeText(v) => {
                        let s = from_utf16(v.as_bytes());
                        writer.write_field(&s).expect(format!("Error writing LargeText '{s}'").as_str())
                    }
                    _ =>
                        panic!("missed writer for {val:?}")
                };
            }
            writer.write_record(None::<&[u8]>).unwrap();
        }
    }
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
    let sql_generator_path = get_env("WSA_TEST_SQL_GENERATOR_PATH");
    let sql_to_csv_path = get_env("WSA_TEST_SQL_TO_CSV_PATH");
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
    info!("work_dir: {work_dir:?}");

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
        .args(["-I", sqlite3ext_h_path.as_str()])
        .arg("-m32")
        .args(["-o", "dtformat.dll"])
        .arg("dtformat.c");

    do_invoke(cmd);

    do_sql_test(&reporter_bin_path, &db_path, &cfg_path, &sql_generator_path, &sql_to_csv_path, &work_dir);
    do_ese_test(&reporter_bin_path, &edb_path, &cfg_path, &work_dir);
}

