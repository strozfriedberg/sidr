use chrono::prelude::*;
use clap::ValueEnum;
use simple_error::SimpleError;
use std::cell::{Cell, RefCell};
use std::fmt::{Display, Formatter, Result as FmtResult};
use serde_json;
use std::fs::File;
use std::io::{self, Write, BufWriter};
use std::ops::IndexMut;
use std::path::{Path, PathBuf};

use crate::utils::*;

#[derive(Clone, Debug, ValueEnum)]
pub enum ReportFormat {
    Json,
    Csv,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum ReportOutput {
    ToFile,
    ToStdout
}

#[derive(Debug, PartialEq)]
pub enum ReportSuffix {
    FileReport,
    ActivityHistory,
    InternetHistory,
    Unknown
}

impl ReportSuffix {
    pub fn get_match(output_type: &str) -> Option<ReportSuffix>{
        match output_type {
            "File_Report" => Some(ReportSuffix::FileReport),
            "Activity_History_Report" => Some(ReportSuffix::ActivityHistory),
            "Internet_History_Report" => Some(ReportSuffix::InternetHistory),
            &_ => Some(ReportSuffix::Unknown)
        }
    }

    // Autogenerating the names from the enum values by deriving Debug is another option.
    // However, if someone decided to change the name of one of these enums,
    // it could break downstream processing.
    pub fn message(&self) -> String {
        match self {
            Self::FileReport => serde_json::to_string("file_report").unwrap(),
            Self::ActivityHistory => serde_json::to_string("activity_history").unwrap(),
            Self::InternetHistory => serde_json::to_string("internet_history").unwrap(),
            Self::Unknown => serde_json::to_string("").unwrap()
        }
    }
}

impl Display for ReportSuffix {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.message())
    }
}

pub struct ReportProducer {
    dir: PathBuf,
    format: ReportFormat,
    report_type: ReportOutput
}

impl ReportProducer {
    pub fn new(dir: &Path, format: ReportFormat, report_type: ReportOutput) -> Self {
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
        let report_suffix = ReportSuffix::get_match(report_suffix);
        let rep: Box<dyn Report> = match self.format {
            ReportFormat::Json => ReportJson::new(&path, self.report_type, report_suffix).map(Box::new)?,
            ReportFormat::Csv => ReportCsv::new(&path, self.report_type, report_suffix).map(Box::new)?,
        };
        Ok((path, rep))
    }
}

pub trait Report {
    fn footer(&mut self) {}
    fn new_record(&mut self);
    fn str_val(&self, f: &str, s: String);
    fn int_val(&self, f: &str, n: u64);
    fn set_field(&self, _: &str) {} // used in csv to generate header
    fn is_some_val_in_record(&self) -> bool;
}

// report json
pub struct ReportJson {
    f: Box<dyn Write + 'static>,
    report_output: ReportOutput,
    report_suffix: Option<ReportSuffix>,
    first_record: Cell<bool>,
    values: RefCell<Vec<String>>,
}

impl ReportJson {
    pub fn new(path: &Path, report_output: ReportOutput, report_suffix: Option<ReportSuffix>) -> Result<Self, SimpleError> {
        match report_output {
            ReportOutput::ToFile => {
                let output: Box<dyn Write> = Box::new(File::create(path).map_err(|e| SimpleError::new(format!("{}", e)))?);
                Ok(ReportJson {
                    f: output,
                    report_output,
                    report_suffix: None,
                    first_record: Cell::new(true),
                    values: RefCell::new(Vec::new()),
                })
            },
            ReportOutput::ToStdout => {
                Ok(ReportJson {
                    f: Box::new(BufWriter::new(io::stdout())),
                    report_output,
                    report_suffix,
                    first_record: Cell::new(true),
                    values: RefCell::new(Vec::new()),
                })
            }
        }
    }

    fn escape(s: String) -> String {
        json_escape(&s)
    }

    pub fn write_values(&mut self) {
        let mut values = self.values.borrow_mut();
        let len = values.len();
        let handle = self.f.as_mut();
        if len > 0 {
            handle.write_all(b"{").unwrap();
        }
        if self.report_output == ReportOutput::ToStdout {
            handle.write_all(format!("{}:{},", serde_json::to_string("report_suffix").unwrap(), self.report_suffix.as_ref().unwrap()).as_bytes()).ok();
        }
        for i in 0..len {
            let v = values.index_mut(i);
            if !v.is_empty() {
                let last = if i == len - 1 { "" } else { "," };
                handle
                    .write_all(format!("{}{}", v, last).as_bytes())
                    .unwrap();
            }
        }
        if len > 0 {
            handle.write_all(b"}").unwrap();
            values.clear();
        }
    }
}

impl Report for ReportJson {
    fn footer(&mut self) {
        self.new_record();
    }

    fn new_record(&mut self) {
        if !self.values.borrow().is_empty() {
            if !self.first_record.get() {
                self.f.as_mut().write_all(b"\n").unwrap();
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
    f: Box<dyn Write + 'static>,
    report_output: ReportOutput,
    report_suffix: Option<ReportSuffix>,
    first_record: Cell<bool>,
    values: RefCell<Vec<(String /*field*/, String /*value*/)>>,
}

impl ReportCsv{
    pub fn new(f: &Path, report_output: ReportOutput, report_suffix: Option<ReportSuffix>) -> Result<Self, SimpleError> {
        match report_output {
            ReportOutput::ToFile => {
                let output: Box<dyn Write> = Box::new(File::create(f).map_err(|e| SimpleError::new(format!("{}", e)))?);
                Ok(ReportCsv {
                    f: output,
                    report_output,
                    report_suffix: None,
                    first_record: Cell::new(true),
                    values: RefCell::new(Vec::new()),
                })
            },
            ReportOutput::ToStdout => {
                Ok(ReportCsv {
                    f: Box::new(BufWriter::new(io::stdout())),
                    report_output,
                    report_suffix,
                    first_record: Cell::new(true),
                    values: RefCell::new(Vec::new()),
                })
            }
        }
    }

    fn escape(s: String) -> String {
        s.replace('\"', "\"\"")
    }

    pub fn write_header(&mut self) {
        let handle = self.f.as_mut();
        if self.report_output == ReportOutput::ToStdout {
            handle.write_all(b"ReportSuffix,").ok();
        }
        let values = self.values.borrow();
        for i in 0..values.len() {
            let v = &values[i];
            if i == values.len() - 1 {
                handle.write_all(v.0.as_bytes()).unwrap();
            } else {
                handle
                    .write_all(format!("{},", v.0).as_bytes())
                    .unwrap();
            }
        }
    }

    pub fn write_values(&mut self) {
        let handle = self.f.as_mut();
        handle.write_all(b"\n").unwrap();

        let mut values = self.values.borrow_mut();
        let len = values.len();
        if self.report_output == ReportOutput::ToStdout {
            handle.write_all(format!("{},", self.report_suffix.as_ref().unwrap()).as_bytes()).ok();
        }
        for i in 0..len {
            let v = values.index_mut(i);
            let last = if i == len - 1 { "" } else { "," };
            if v.1.is_empty() {
                handle
                    .write_all(last.to_string().as_bytes())
                    .unwrap();
            } else {
                handle
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
    fn footer(&mut self) {
        self.new_record();
    }

    fn new_record(&mut self) {
        // at least 1 value was recorded?
        if self.is_some_val_in_record() {
            if self.first_record.get() {
                self.write_header();
                self.first_record.set(false);
            }
            self.write_values();
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
    let report_type = ReportOutput::ToFile;
    let report_suffix = None;
    {
        let mut r = ReportCsv::new(p, report_type, report_suffix).unwrap();
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
    let report_type = ReportOutput::ToFile;
    let report_suffix = Some(ReportSuffix::FileReport);
    {
        let mut r = ReportJson::new(p, report_type, report_suffix).unwrap();
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

#[test]
fn test_report_suffix() {
    let report_suffix = Some(ReportSuffix::FileReport);
    assert_eq!(ReportSuffix::get_match("File_Report"), report_suffix);
    assert_ne!(ReportSuffix::get_match("Activity"), report_suffix);

    assert_eq!(ReportSuffix::message(report_suffix.as_ref().unwrap()), serde_json::to_string("file_report").unwrap());
    assert_eq!(ReportSuffix::message(&ReportSuffix::ActivityHistory), serde_json::to_string("activity_history").unwrap());
    assert_eq!(ReportSuffix::message(&ReportSuffix::InternetHistory), serde_json::to_string("internet_history").unwrap());
    assert_eq!(ReportSuffix::message(&ReportSuffix::Unknown), serde_json::to_string("").unwrap());
}
