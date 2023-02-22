
use std::cell::{RefCell, Cell};
use std::fs::File;
use std::io::Write;
use std::ops::IndexMut;
use std::path::{Path, PathBuf};
use simple_error::SimpleError;

pub enum ReportFormat {
    Json,
    Csv
}

pub struct ReportProducer {
    dir: PathBuf,
    format: ReportFormat
}

impl ReportProducer {
    pub fn new(dir: &Path, format: ReportFormat) -> Self {
        if !dir.exists() {
            std::fs::create_dir(dir)
                .expect(&format!("Can't create directory \"{}\"", dir.to_string_lossy()));
        }
        ReportProducer {
            dir: dir.to_path_buf(),
            format
        }
    }

    pub fn new_report(&self, dbpath: &Path, report_suffix: &str) -> Result<(PathBuf, Box<dyn Report>), SimpleError> {
        let ext = match self.format {
            ReportFormat::Json => "json",
            ReportFormat::Csv => "csv"
        };
        let path = self.dir.join(format!("{}.{}.{}",
            dbpath.file_name().unwrap().to_string_lossy(),
            report_suffix,
            ext));
        let rep : Box<dyn Report> = match self.format {
            ReportFormat::Json => ReportJson::new(&path).map(|r| Box::new(r))?,
            ReportFormat::Csv => ReportCsv::new(&path).map(|r| Box::new(r))?
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
}

// report json
pub struct ReportJson {
    f: RefCell<File>,
    first_record: Cell<bool>,
    values: RefCell<Vec<String>>,
}

impl ReportJson {
    pub fn new(f: &Path) -> Result<Self, SimpleError> {
        let mut f = File::create(f).map_err(|e| SimpleError::new(format!("{}", e)))?;
        f.write(b"[").unwrap();
        Ok(ReportJson {
            f: RefCell::new(f),
            first_record: Cell::new(true),
            values: RefCell::new(Vec::new())
        })
    }

    fn escape(s: String) -> String {
        s.replace("\\", "\\\\").replace("\"", "\\\"")
    }

    pub fn write_values(&self) {
        let mut values = self.values.borrow_mut();
        let len = values.len();
        if len > 0 {
            self.f.borrow_mut().write(b"{").unwrap();
        }
        for i in 0..len {
            let v = values.index_mut(i);
            if !v.is_empty() {
                let last = if i == len-1 { "" } else { "," };
                self.f.borrow_mut().write(format!("{}{}", v, last).as_bytes()).unwrap();
            }
        }
        if len > 0 {
            self.f.borrow_mut().write(b"}").unwrap();
            values.clear();
        }
    }
}

impl Report for ReportJson {
    fn footer(&self) {
        self.new_record();
        self.f.borrow_mut().write(b"]").unwrap();
    }

    fn new_record(&self) {
        if !self.values.borrow().is_empty() {
            if !self.first_record.get() {
                self.f.borrow_mut().write(b",\n").unwrap();
            } else {
                self.first_record.set(false);
            }
            self.write_values();
        }
    }

    fn str_val(&self, f: &str, s: String) {
        self.values.borrow_mut().push(format!("\"{}\":\"{}\"", f, ReportJson::escape(s)));
    }
    
    fn int_val(&self, f: &str, n: u64) {
        self.values.borrow_mut().push(format!("\"{}\":{}", f, n));
    }
}

impl Drop for ReportJson {
    fn drop(&mut self) {
        self.footer();
    }
}

// report csv
pub struct ReportCsv {
    f: RefCell<File>,
    first_record: Cell<bool>,
    values: RefCell<Vec<(String/*field*/, String/*value*/)>>,
}

impl ReportCsv {
    pub fn new(f: &Path) -> Result<Self, SimpleError> {
        let f = File::create(f).map_err(|e| SimpleError::new(format!("{}", e)))?;
        Ok(ReportCsv {
            f: RefCell::new(f),
            first_record: Cell::new(true),
            values: RefCell::new(Vec::new()),
        })
    }

    fn escape(s: String) -> String {
        s.replace("\"", "\"\"")
    }

    pub fn write_header(&self) {
        let values = self.values.borrow();
        for i in 0..values.len() {
            let v = &values[i];
            if i == values.len()-1 {
                self.f.borrow_mut().write(v.0.as_bytes()).unwrap();
            } else {
                self.f.borrow_mut().write(format!("{},", v.0).as_bytes()).unwrap();
            }
        }
    }

    pub fn write_values(&self) {
        let mut values = self.values.borrow_mut();
        let len = values.len();
        for i in 0..len {
            let v = values.index_mut(i);
            let last = if i == len-1 { "" } else { "," };
            if v.1.is_empty() {
                self.f.borrow_mut().write(format!("{}", last).as_bytes()).unwrap();
            } else {
                self.f.borrow_mut().write(format!("{}{}", v.1, last).as_bytes()).unwrap();
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
        let some_values = self.values.borrow().iter().find(|i| !i.1.is_empty()).is_some();
        if some_values {
            if self.first_record.get() {
                self.write_header();
                self.first_record.set(false);
            }
            self.f.borrow_mut().write(b"\n").unwrap();
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
        let r = ReportCsv::new(&p).unwrap();
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
    let data = std::fs::read_to_string(&p).unwrap();
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
    std::fs::remove_file(&p).unwrap();
}

#[test]
pub fn test_report_json() {
    let p = Path::new("test.json");
    {
        let r = ReportJson::new(&p).unwrap();
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
    let data = std::fs::read_to_string(&p).unwrap();
    let expected = r#"[{"int_field":0,"str_field":"string0_with_escapes_here1\"here2\\"},
{"int_field":1},
{"str_field":"string2"},
{"int_field":3},
{"str_field":"string4"},
{"int_field":5},
{"str_field":"string6"},
{"int_field":7},
{"str_field":"string8"},
{"int_field":9}]"#;
    assert_eq!(data, expected);
    std::fs::remove_file(&p).unwrap();
}