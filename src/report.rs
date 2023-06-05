use std::borrow::BorrowMut;
use chrono::prelude::*;
use clap::ValueEnum;
use simple_error::SimpleError;
use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::{self, Write};
use std::ops::IndexMut;
use std::path::{Path, PathBuf};

use crate::utils::*;

#[derive(Clone, Debug, ValueEnum)]
pub enum ReportFormat {
    Json,
    Csv,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ReportType {
    ToFile,
    ToStdout
}

pub struct ReportProducer {
    dir: PathBuf,
    format: ReportFormat,
    report_type: ReportType
}

impl ReportProducer {
    pub fn new(dir: &Path, format: ReportFormat, report_type: ReportType) -> Self {
        if !dir.exists() {
            std::fs::create_dir(dir)
                .unwrap_or_else(|_| panic!("Can't create directory \"{}\"", dir.to_string_lossy()));
        }
        ReportProducer {
            dir: dir.to_path_buf(),
            format,
            report_type,
        }
    }

    pub fn new_report(
        &self,
        _dbpath: &Path,
        recovered_hostname: &str,
        report_suffix: &str,
    ) -> Result<(PathBuf, Box<dyn Report>), SimpleError> {
        let ext = match self.format {
            ReportFormat::Json => "json",
            ReportFormat::Csv => "csv",
        };
        let date_time_now: DateTime<Utc> = Utc::now();
        let path = self.dir.join(format!(
            "{}_{}_{}.{}",
            recovered_hostname,
            report_suffix,
            date_time_now.format("%Y%m%d_%H%M%S%.f"),
            ext
        ));
        let rep: Box<dyn Report> = match self.format {
            ReportFormat::Json => ReportJson::new(&path, self.report_type).map(Box::new)?,
            ReportFormat::Csv => ReportCsv::new(&path, self.report_type).map(Box::new)?,
        };
        Ok((path, rep))
    }
}

pub trait Report {
    fn footer(&self) {}
    fn new_record(&self);
    fn str_val(&self, f: &str, s: String);
    fn int_val(&self, f: &str, n: u64);
    fn set_field(&self, _: &str) {} // used in csv to generate header
    fn is_some_val_in_record(&self) -> bool;
}

fn get_stdout_handle() -> std::io::StdoutLock<'static> {
    let stdout = io::stdout();
    stdout.lock()
}

// report json
pub struct ReportJson{
    f: Option<RefCell<File>>,
    report_type: ReportType,
    first_record: Cell<bool>,
    values: RefCell<Vec<String>>,
}

impl ReportJson{
    pub fn new(f: &Path, report_type: ReportType) -> Result<Self, SimpleError> {
        match report_type {
            ReportType::ToFile => {
                let f = File::create(f).map_err(|e| SimpleError::new(format!("{}", e)))?;
                Ok(ReportJson {
                    f: Some(RefCell::new(f)),
                    report_type,
                    first_record: Cell::new(true),
                    values: RefCell::new(Vec::new()),
                })
            },
            ReportType::ToStdout => {
                Ok(ReportJson {
                    f: None,
                    report_type,
                    first_record: Cell::new(true),
                    values: RefCell::new(Vec::new()),
                })
            }
        }
    }

    fn escape(s: String) -> String {
        json_escape(&s)
    }

    pub fn write_values(&self) {
        let mut values = self.values.borrow_mut();
        let len = values.len();
        if len > 0 {
            self.f.as_ref().unwrap().borrow_mut().write_all(b"{").unwrap();
        }
        for i in 0..len {
            let v = values.index_mut(i);
            if !v.is_empty() {
                let last = if i == len - 1 { "" } else { "," };
                self.f
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .write_all(format!("{}{}", v, last).as_bytes())
                    .unwrap();
            }
        }
        if len > 0 {
            self.f.as_ref().unwrap().borrow_mut().write_all(b"}").unwrap();
            values.clear();
        }
    }
}

impl Report for ReportJson {
    fn footer(&self) {
        self.new_record();
    }

    fn new_record(&self) {
        if !self.values.borrow().is_empty() {
            if !self.first_record.get() {
                self.f.as_ref().unwrap().borrow_mut().write_all(b"\n").unwrap();
            } else {
                self.first_record.set(false);
            }
            self.write_values();
        }
    }

    fn str_val(&self, f: &str, s: String) {
        self.values
            .borrow_mut()
            .push(format!("\"{}\":{}", f, ReportJson::escape(s)));
    }

    fn int_val(&self, f: &str, n: u64) {
        self.values.borrow_mut().push(format!("\"{}\":{}", f, n));
    }

    fn is_some_val_in_record(&self) -> bool {
        !self.values.borrow().is_empty()
    }
}

impl Drop for ReportJson {
    fn drop(&mut self) {
        self.footer();
    }
}

// report csv
pub struct ReportCsv{
    f: Option<RefCell<File>>,
    report_type: ReportType,
    first_record: Cell<bool>,
    values: RefCell<Vec<(String /*field*/, String /*value*/)>>,
}

impl ReportCsv{
    pub fn new(f: &Path, report_type: ReportType) -> Result<Self, SimpleError> {
        match report_type {
            ReportType::ToFile => {
                let f = File::create(f).map_err(|e| SimpleError::new(format!("{}", e)))?;
                Ok(ReportCsv {
                    f: Some(RefCell::new(f)),
                    report_type,
                    first_record: Cell::new(true),
                    values: RefCell::new(Vec::new()),
                })
            },
            ReportType::ToStdout => {
                Ok(ReportCsv {
                    f: None,
                    report_type,
                    first_record: Cell::new(true),
                    values: RefCell::new(Vec::new()),
                })
            }
        }
    }

    fn escape(s: String) -> String {
        s.replace('\"', "\"\"")
    }

    pub fn write_header_stdout(&self) {
        let values = self.values.borrow();
        let mut handle = get_stdout_handle();
        for i in 0..values.len() {
            let v = &values[i];
            if i == values.len() - 1 {
                handle.write_all(v.0.as_bytes()).unwrap();
            } else {
                handle.write_all(format!("{},", v.0).as_bytes());
            }
        }
        handle.write_all(b"\n").unwrap();
    }

    pub fn write_header_file(&self) {
        let values = self.values.borrow();
        for i in 0..values.len() {
            let v = &values[i];
            if i == values.len() - 1 {
                self.f.as_ref().unwrap().borrow_mut().write_all(v.0.as_bytes()).unwrap();
            } else {
                self.f
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .write_all(format!("{},", v.0).as_bytes())
                    .unwrap();
            }
        }
    }

    pub fn write_values_stdout(&self) {
        let mut values = self.values.borrow_mut();
        let len = values.len();
        let mut handle = get_stdout_handle();
        println!("To stdout is used: {:?}", self.report_type);
        for i in 0..len {
            let v = values.index_mut(i);
            let last = if i == len - 1 { "" } else { "," };
            if v.1.is_empty() {
                handle.write_all(format!("{}{}", v.1, last).as_bytes());
            } else {
                handle.write_all(format!("{}{}", v.1, last).as_bytes());
                v.1.clear();
            }
        }
    }

    pub fn write_values_file(&self) {
        let mut values = self.values.borrow_mut();
        let len = values.len();
        println!("To file is used: {:?}", self.report_type);
        for i in 0..len {
            let v = values.index_mut(i);
            let last = if i == len - 1 { "" } else { "," };
            if v.1.is_empty() {
                self.f
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .write_all(last.to_string().as_bytes())
                    .unwrap();
            } else {
                self.f
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .write_all(format!("{}{}", v.1, last).as_bytes())
                    .unwrap();
                v.1.clear();
            }
        }
    }

    pub fn update_field_with_value(&self, f: &str, v: String) {
        let mut values = self.values.borrow_mut();
        if let Some(found) = values.iter_mut().find(|i| i.0 == f) {
            found.1 = v;
        } else {
            values.push((f.into(), v));
        }
    }
}

impl Report for ReportCsv {
    fn footer(&self) {
        self.new_record();
    }

    fn new_record(&self) {
        // at least 1 value was recorded?
        if self.is_some_val_in_record() {
            if self.first_record.get() {
                match self.report_type {
                    ReportType::ToFile => {
                        self.write_header_file();
                        self.f.as_ref().unwrap().borrow_mut().write_all(b"\n").unwrap();
                    },
                    ReportType::ToStdout => {
                        self.write_header_stdout()
                    }
                }
                self.first_record.set(false);
            }
            match self.report_type {
                ReportType::ToFile => self.write_values_file(),
                ReportType::ToStdout => self.write_values_stdout()
            }

        }
    }

    fn str_val(&self, f: &str, s: String) {
        self.update_field_with_value(f, format!("\"{}\"", ReportCsv::escape(s)));
    }

    fn int_val(&self, f: &str, n: u64) {
        self.update_field_with_value(f, n.to_string());
    }

    fn set_field(&self, f: &str) {
        // set field with empty value to record field name
        self.update_field_with_value(f, "".to_string());
    }

    fn is_some_val_in_record(&self) -> bool {
        self.values.borrow().iter().any(|i| !i.1.is_empty())
    }
}

impl Drop for ReportCsv {
    fn drop(&mut self) {
        self.footer();
    }
}

#[test]
pub fn test_report_csv() {
    let p = Path::new("test.csv");
    {
        let r = ReportCsv::new(p).unwrap();
        r.set_field("int_field");
        r.set_field("str_field");
        r.int_val("int_field", 0);
        r.str_val("str_field", "string0".into());
        for i in 1..10 {
            r.new_record();
            if i % 2 == 0 {
                r.str_val("str_field", format!("string{}", i));
            } else {
                r.int_val("int_field", i);
            }
        }
    }
    let data = std::fs::read_to_string(p).unwrap();
    let expected = r#"int_field,str_field
0,"string0"
1,
,"string2"
3,
,"string4"
5,
,"string6"
7,
,"string8"
9,"#;
    assert_eq!(data, expected);
    std::fs::remove_file(p).unwrap();
}

#[test]
pub fn test_report_jsonl() {
    let p = Path::new("test.json");
    {
        let r = ReportJson::new(p).unwrap();
        r.int_val("int_field", 0);
        r.str_val("str_field", "string0_with_escapes_here1\"here2\\".into());
        for i in 1..10 {
            r.new_record();
            if i % 2 == 0 {
                r.str_val("str_field", format!("string{}", i));
            } else {
                r.int_val("int_field", i);
            }
        }
    }
    let data = std::fs::read_to_string(p).unwrap();
    let expected = r#"{"int_field":0,"str_field":"string0_with_escapes_here1\"here2\\"}
{"int_field":1}
{"str_field":"string2"}
{"int_field":3}
{"str_field":"string4"}
{"int_field":5}
{"str_field":"string6"}
{"int_field":7}
{"str_field":"string8"}
{"int_field":9}"#;
    assert_eq!(data, expected);
    std::fs::remove_file(p).unwrap();
}
