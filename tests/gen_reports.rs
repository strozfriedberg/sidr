
#[cfg(test)]
use std::{
    env,
    fs,
    process::{Command, Stdio},
};

use ::function_name::named;
use camino::Utf8PathBuf as PathBuf;
use env_logger::{self, Target};
use glob::glob;
use log::info;
use tempdir::TempDir;

use wsa_lib::utils::{format_date_time, from_utf16, get_date_time_from_filetime};
use wsa_lib::{utils, ColumnType, ReportsCfg};

macro_rules! function_path {
    () => {
        concat!(module_path!(), "::", function_name!())
    };
}

fn glob_vec_path(pattern: &str) -> Vec<PathBuf> {
    glob(pattern)
        .unwrap()
        .map(|p| p.unwrap())
        .map(|p| PathBuf::from_path_buf(p).unwrap())
        .collect()
}

fn glob_vec_string(pattern: &str) -> Vec<String> {
    glob_vec_path(pattern)
        .into_iter()
        .map(|p| p.as_str().to_string())
        .collect()
}

fn glob_vec_names(pattern: &str) -> Vec<String> {
    glob_vec_path(pattern)
        .into_iter()
        .map(|p| p.file_name().unwrap().to_string())
        .collect()
}

fn get_env(var: &str) -> String {
    env::var(var).expect(format!("Error getting environment variable '{var}'").as_str())
}

#[named]
fn remove_files(pattern: &str) {
    info!("{}", function_path!());
    let paths = glob_vec_path(pattern);
    paths
        .iter()
        .for_each(|p| fs::remove_file(p).expect(&format!("remove '{}' failed", p)));
}

#[named]
fn do_invoke(cmd: &mut Command) {
    info!("{}", function_path!());
    let args: Vec<&str> = cmd
        .get_args()
        .into_iter()
        .map(|a| a.to_str().unwrap())
        .collect();
    info!(
        "cmd '{} {}'",
        cmd.get_program().to_str().unwrap(),
        args.join(" ")
    );
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

#[named]
fn generate_reports(reporter_bin: &str, db_path: &str, common_args: &Vec<&str>) {
    info!("{}", function_path!());
    let mut cmd = Command::new(&reporter_bin);
    let cmd = cmd.args(&*common_args).args(["csv", db_path]);

    do_invoke(cmd);

    let mut cmd = Command::new(&reporter_bin);
    let cmd = cmd.args(&*common_args).args(["json", db_path]);

    do_invoke(cmd);
}

fn do_generate(reporter_bin: &str, db_path: &str, rep_dir: &str, specific_args: &Vec<&str>) {
    remove_files(format!("{}/*.csv", rep_dir).as_str());
    remove_files(format!("{}/*.json", rep_dir).as_str());

    let mut common_args = vec!["--outdir", rep_dir];
    common_args.extend(specific_args);
    common_args.push("--format");
    generate_reports(&reporter_bin, &db_path, &common_args);
}

#[test]
#[named]
fn compare_generated_reports() {
    env_logger::builder().target(Target::Stderr).init();

    info!("{}", function_path!());

    let bin_dir: PathBuf = ["target", "debug"].iter().collect();
    let sidr_bin = bin_dir.join("sidr");
    let ext_cfg_bin = bin_dir.join("external_cfg");
    let db_path = get_env("WSA_TEST_DB_PATH");
    let cfg_path = get_env("WSA_TEST_CONFIGURATION_PATH");
    let work_dir_name = format!("{}_testing", ext_cfg_bin.file_name().unwrap());
    let work_temp_dir = TempDir::new(work_dir_name.as_str()).expect("{work_dir_name} creation");
    let _work_dir_keeper;
    let work_dir = if env::var("KEEP_TEMP_WORK_DIR").is_ok() {
        _work_dir_keeper = work_temp_dir.into_path();
        _work_dir_keeper.as_path()
    } else {
        work_temp_dir.path()
    };
    let sidr_dir = PathBuf::from_path_buf(work_dir.join("sidr")).unwrap();
    let ext_cfg_dir: PathBuf = PathBuf::from_path_buf(work_dir.join("ext_cfg")).unwrap();

    info!("db_path: {db_path}");
    info!("cfg_path: {cfg_path}");
    info!("sidr_dir: {sidr_dir}");
    info!("ext_cfg_dir: {ext_cfg_dir}");

    fs::create_dir(&sidr_dir).expect(&format!("could not create '{}'", sidr_dir));
    fs::create_dir(&ext_cfg_dir).expect(&format!("could not create '{}'", ext_cfg_dir));

    do_generate(sidr_bin.as_str(), db_path.as_str(), sidr_dir.as_str(), &vec![]);
    do_generate(ext_cfg_bin.as_str(), db_path.as_str(), ext_cfg_dir.as_str(), &vec!["--cfg-path", &cfg_path]);
}
