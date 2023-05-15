#![feature(let_chains)]
#[cfg(test)]
use std::{
    env, fs,
    process::{Command, Stdio},
};

use ::function_name::named;
use camino::Utf8PathBuf as PathBuf;
use csv::{Reader, StringRecordIter};
use env_logger::{self, Target};
use log::info;
use std::path::Path as StdPath;
use tempdir::TempDir;
use walkdir::{DirEntry, Error, WalkDir};

macro_rules! function_path {
    () => {
        concat!(module_path!(), "::", function_name!())
    };
}

fn get_dir<P: AsRef<StdPath>>(path: P, ext: &str) -> Vec<PathBuf> {
    fn get_filename(f: &Result<DirEntry, Error>) -> &str {
        f.as_ref().unwrap().file_name().to_str().unwrap()
    }

    WalkDir::new(path)
        .same_file_system(true)
        .into_iter()
        .filter_map(|ref f| {
            if get_filename(f).ends_with(ext) {
                Some(PathBuf::from(get_filename(f)))
            } else {
                None
            }
        })
        .collect()
}

fn get_env(var: &str) -> String {
    env::var(var).unwrap_or_else(|_| panic!("Error getting environment variable '{var}'"))
}

#[named]
fn do_invoke(cmd: &mut Command) {
    info!("{}", function_path!());
    let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
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
        .unwrap_or_else(|_| panic!("'{cmd:?}' command failed to start"));

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
    let mut cmd = Command::new(reporter_bin);
    let cmd = cmd.args(common_args).args(["csv", db_path]);

    do_invoke(cmd);

    let mut cmd = Command::new(reporter_bin);
    let cmd = cmd.args(common_args).args(["json", db_path]);

    do_invoke(cmd);
}

fn do_generate(reporter_bin: &str, db_path: &str, rep_dir: &str, specific_args: &Vec<&str>) {
    let mut common_args = vec!["--outdir", rep_dir];
    common_args.extend(specific_args);
    common_args.push("--format");
    generate_reports(reporter_bin, db_path, &common_args);
}

fn compare_iters(sidr_iter: &mut StringRecordIter, ext_iter: &mut StringRecordIter, msg: &str) {
    if !itertools::equal(sidr_iter.clone(), ext_iter.clone()) {
        let mut i = 0;
        for (s, e) in sidr_iter.zip(ext_iter) {
            i += 1;
            if s != e {
                println!("{i}. '{s}' != '{e}'")
            }
        }
        panic!("{}", msg);
    }
}

fn do_compare_csv(sidr_path: &str, ext_cfg_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir_sidr = get_dir(sidr_path, ".csv");
    let dir_ext_cfg = get_dir(ext_cfg_path, ".csv");
    // let pairs: Vec<_> = dir_sidr.iter().zip(dir_ext_cfg.iter()).collect();
    // println!("{pairs:?}");

    for (sidr, ext_cfg) in dir_sidr.iter().zip(dir_ext_cfg.iter()) {
        println!("{sidr} <-> {ext_cfg}");

        let sidr = PathBuf::from_iter([sidr_path.clone(), sidr.as_str()].iter());
        let mut sidr_reader = Reader::from_path(sidr)?;
        let ext_cfg = PathBuf::from_iter([ext_cfg_path.clone(), ext_cfg.as_str()].iter());
        let mut ext_cfg_reader = Reader::from_path(ext_cfg)?;
        let mut sidr_iter = sidr_reader.headers()?.iter();
        let mut ext_iter = ext_cfg_reader.headers()?.iter();

        compare_iters(&mut sidr_iter, &mut ext_iter, "headers are not equal");

        let mut sidr_reader = sidr_reader.into_records();
        let mut ext_cfg_reader = ext_cfg_reader.into_records();
        let mut rec_no = 0;

        while let Some(sid_rec) = sidr_reader.next() && let Some(ext_rec) = ext_cfg_reader.next() {
            let sid_rec = sid_rec?;
            let mut sid_fld = sid_rec.iter();
            let ext_rec = ext_rec?;
            let mut ext_fld = ext_rec.iter();

            rec_no += 1;
            compare_iters(&mut sid_fld, &mut ext_fld, &format!("data differs at {rec_no}"));
        }
    }

    Ok(())
}

fn do_compare(sidr_path: &str, ext_cfg_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    do_compare_csv(sidr_path, ext_cfg_path)
}

#[test]
#[named]
fn compare_generated_reports() {
    env_logger::builder().target(Target::Stderr).init();

    info!("{}", function_path!());

    let bin_root = PathBuf::from("target");
    let sidr_bin = bin_root.join("release").join("sidr");
    #[cfg(not(debug_assertions))]
    let mut ext_cfg_bin = bin_root.join("release");
    #[cfg(debug_assertions)]
    let mut ext_cfg_bin = bin_root.join("debug");
    ext_cfg_bin = ext_cfg_bin.join("external_cfg");
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

    fs::create_dir(&sidr_dir).unwrap_or_else(|_| panic!("could not create '{}'", sidr_dir));
    fs::create_dir(&ext_cfg_dir).unwrap_or_else(|_| panic!("could not create '{}'", ext_cfg_dir));

    do_generate(
        sidr_bin.as_str(),
        db_path.as_str(),
        sidr_dir.as_str(),
        &vec![],
    );
    do_generate(
        ext_cfg_bin.as_str(),
        db_path.as_str(),
        ext_cfg_dir.as_str(),
        &vec!["--cfg-path", &cfg_path],
    );

    do_compare(sidr_dir.as_str(), ext_cfg_dir.as_str()).expect("compare failed");
}
