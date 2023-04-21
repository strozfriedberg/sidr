#![allow(non_upper_case_globals)]
#[warn(non_camel_case_types)]
pub mod report;
#[allow(non_camel_case_types)]
pub mod utils;

use ::function_name::named;
use evalexpr;
use log::{debug, info, trace};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str, string::String};

macro_rules! function_path {
    () => {
        concat!(module_path!(), "::", function_name!())
    };
}

//---------------------------------------------------
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ColumnType {
    String,
    Integer,
    DateTime,
    GUID,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub constraint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnPair {
    pub title: String,
    pub kind: ColumnType,
    pub edb: Column,
    pub sql: Column,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OutputFormat {
    Csv,
    Json,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportCfg {
    pub title: String,
    pub columns: Vec<ColumnPair>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportsCfg {
    pub table_edb: String,
    pub table_sql: String,
    pub output_format: OutputFormat,
    pub output_dir: String,
    pub reports: Vec<ReportCfg>,
}

//--------------------------------------------------------------------
use chrono::{DateTime, TimeZone, Utc};

type FldId = String;

#[derive(PartialEq, Debug, Clone)]
pub struct ConstrainedField {
    name: String,
    constraint: Option<String>,
    idx: usize,
}

impl ConstrainedField {
    fn new(name: &str, constraint: &Option<String>, idx: usize) -> Self {
        Self {
            name: name.to_string(),
            constraint: if let Some(s) = constraint {
                Some(s.to_string())
            } else {
                None
            },
            idx,
        }
    }
}

pub trait FieldReader {
    fn get_used_columns(&mut self, columns: &Vec<ColumnPair>) -> Vec<ConstrainedField>;
    fn init(&mut self) -> bool;
    fn next(&mut self) -> bool;
    fn get_int(&mut self, id: &FldId) -> Option<i64>;
    fn get_str(&mut self, id: &FldId) -> Option<String>;
    fn get_guid(&mut self, id: &FldId) -> Option<String>;
    fn get_datetime(&mut self, id: &FldId) -> Option<DateTime<Utc>>;
}

//--------------------------------------------------------------------
use ese_parser_lib::vartime::{get_date_time_from_filetime, VariantTimeToSystemTime, SYSTEMTIME};
use ese_parser_lib::{ese_parser::EseParser, ese_trait::*};
use num;
use std::{fs::File, io::BufReader};
use utils::{find_guid, from_utf16};

const CACHE_SIZE_ENTRIES: usize = 10;

fn field_size(col_type: u32, size: u32) -> u32 {
    match col_type {
        ESE_coltypUnsignedByte => 1,
        ESE_coltypShort => 2,
        ESE_coltypLong => 4,
        ESE_coltypCurrency => 8,
        ESE_coltypIEEESingle => 4,
        ESE_coltypIEEEDouble => 8,
        ESE_coltypDateTime => 8,
        ESE_coltypBinary => size,
        ESE_coltypText => 0,
        ESE_coltypLongBinary => size,
        ESE_coltypLongText => 0,
        ESE_coltypUnsignedLong => 4,
        ESE_coltypLongLong => 8,
        ESE_coltypGUID => 16,
        ESE_coltypUnsignedShort => 2,
        _ => panic!("{col_type} - unknown field type"),
    }
}

pub struct EseReader {
    jdb: Box<EseParser<BufReader<File>>>,
    filename: String,
    table: u64,
    tablename: String,
    col_infos: HashMap<String, (u32, u32)>,
}

fn get_column<T: FromBytes + num::NumCast>(
    jdb: &dyn EseDb,
    table: u64,
    column: u32,
) -> Option<i64> {
    match jdb.get_column(table, column) {
        Ok(r) => match r {
            Some(v) => num::cast::<_, i64>(T::from_bytes(&v)),
            None => None,
        },
        Err(e) => panic!("Error: {e}"),
    }
}

impl EseReader {
    #[named]
    pub fn new(filename: &str, tablename: &str) -> Self {
        info!("{}: {filename}/{tablename}", function_path!());
        let jdb = Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, filename).unwrap());
        let table = jdb.open_table(tablename).unwrap();

        EseReader {
            jdb,
            table,
            tablename: tablename.to_string(),
            filename: filename.to_string(),
            col_infos: HashMap::<String, (u32, u32)>::new(),
        }
    }
}

impl FieldReader for EseReader {
    #[named]
    fn get_used_columns(&mut self, columns: &Vec<ColumnPair>) -> Vec<ConstrainedField> {
        trace!("{}", function_path!());
        let mut used_cols = Vec::<ConstrainedField>::with_capacity(columns.len());
        let tablename = &self.tablename;
        let cols = self.jdb.get_columns(tablename).unwrap();
        let col_infos = &mut self.col_infos;
        for col_pair in columns {
            let name = col_pair.edb.name.clone();
            let mut idx = 0_usize;

            if !name.is_empty() {
                match cols.iter().find(|col| col.name == name) {
                    Some(col_info) => {
                        col_infos.insert(
                            col_pair.title.clone(),
                            (col_info.id, field_size(col_info.typ, col_info.cbmax)),
                        );
                        used_cols.push(ConstrainedField::new(
                            &col_pair.title,
                            &col_pair.edb.constraint,
                            idx,
                        ));
                        idx += 1;
                    }
                    None => panic!(
                        "Could not find '{name}' column in '{tablename}' table in '{}'",
                        self.filename
                    ),
                }
            }
        }

        info!("{}: {used_cols:?}", function_path!());
        used_cols
    }

    #[named]
    fn init(&mut self) -> bool {
        trace!("{}", function_path!());
        self.jdb.move_row(self.table, ESE_MoveFirst).unwrap()
    }

    //#[named]
    fn next(&mut self) -> bool {
        //trace!("{}", function_path!());
        self.jdb.move_row(self.table, ESE_MoveNext).unwrap()
    }

    fn get_datetime(&mut self, id: &FldId) -> Option<DateTime<Utc>> {
        if !self.col_infos.contains_key(id) {
            return None;
        }

        let r = self
            .jdb
            .get_column(self.table, self.col_infos[id].0)
            .unwrap();
        if let Some(v) = r {
            if let Ok(val) = v.clone().try_into() {
                let vartime = f64::from_le_bytes(val);
                let mut st = SYSTEMTIME::default();
                if VariantTimeToSystemTime(vartime, &mut st) {
                    let datetime = Utc
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
                    return Some(datetime);
                } else {
                    let filetime = u64::from_le_bytes(v.try_into().unwrap());
                    let datetime = get_date_time_from_filetime(filetime);
                    return Some(datetime);
                }
            }
        }
        None
    }

    fn get_int(&mut self, id: &FldId) -> Option<i64> {
        if !self.col_infos.contains_key(id) {
            return None;
        }
        let (fld_id, fld_size) = self.col_infos[id];
        match fld_size {
            1 => get_column::<i8>(&*self.jdb, self.table, fld_id),
            2 => get_column::<i16>(&*self.jdb, self.table, fld_id),
            4 => get_column::<i32>(&*self.jdb, self.table, fld_id),
            8 => get_column::<i64>(&*self.jdb, self.table, fld_id),
            _ => panic!("{id} - {fld_size} wrong size of int field"),
        }
    }

    fn get_str(&mut self, id: &FldId) -> Option<String> {
        if !self.col_infos.contains_key(id) {
            return None;
        }
        match self.jdb.get_column(self.table, self.col_infos[id].0) {
            Ok(r) => match r {
                Some(v) => Some(from_utf16(v.as_slice())),
                None => None,
            },
            Err(e) => panic!("{id} - error: {e}"),
        }
    }

    fn get_guid(&mut self, _id: &FldId) -> Option<String> {
        todo!()
    }
}

//--------------------------------------------------------------------
extern crate sqlite3_sys as ffi;
use multimap::MultiMap;
use owning_ref::OwningHandle;
use sqlite::{Connection, OpenFlags, State, Statement};
use std::cell::RefCell;

type ColCode = String;
type ColName = String;
type CodeColDict = MultiMap<ColCode, ConstrainedField>;
type SqlRow = HashMap<ColName, sqlite::Value>;
type Session<'connection> = OwningHandle<Box<Connection>, Box<Statement<'connection>>>;

pub struct SqlReader<'a> {
    last_work_id: u64,
    code_col_dict: CodeColDict,
    row_values: RefCell<SqlRow>,
    session: Session<'a>,
}

impl SqlReader<'_> {
    pub fn new_(db_path: &str) -> Self {
        let conn = Connection::open_with_flags(db_path, OpenFlags::new().set_read_only()).unwrap();
        let sql = "select WorkId, * from SystemIndex_1_PropertyStore order by WorkId";
        let mut session = Session::new_with_fn(Box::new(conn), unsafe {
            |x| Box::new((*x).prepare(sql).unwrap())
        });

        SqlReader {
            session,
            row_values: RefCell::new(SqlRow::new()),
            code_col_dict: CodeColDict::new(),
            last_work_id: 0,
        }
    }

    fn first_row(&mut self) -> bool {
        self.last_work_id = 0;
        self.session.reset().is_ok()
    }

    fn next_row(&mut self) -> bool {
        if let Ok(result) = self.session.next() {
            result == State::Row
        } else {
            false
        }
    }

    fn read<T: sqlite::ReadableWithIndex, U: sqlite::ColumnIndex>(
        &self,
        index: U,
    ) -> sqlite::Result<T> {
        self.session.read(index)
    }

    fn store_value(&mut self, code: &ColCode) {
        let code_col = &self.code_col_dict;

        if code_col.contains_key(code) {
            let value = match self.read::<sqlite::Value, _>("Value") {
                Ok(x) => x,
                Err(e) => panic!("{}", e),
            };

            for cc in code_col.get_vec(code).unwrap() {
                let col_name = &cc.name;
                debug!("{col_name} => {value:?}");
                self.row_values
                    .borrow_mut()
                    .insert(col_name.to_string(), value.clone());
            }
        } else {
            //debug!("skip code {code}");
        }
    }

    fn get_value(&self, col_name: &ColName) -> Option<sqlite::Value> {
        if let Some(x) = self.row_values.borrow().get(col_name).cloned() {
            return Some(x);
        }
        None
    }
}

impl<'a> FieldReader for SqlReader<'a> {
    #[named]
    fn get_used_columns(&mut self, columns: &Vec<ColumnPair>) -> Vec<ConstrainedField> {
        trace!("{}", function_path!());

        let code_col_dict: CodeColDict = CodeColDict::from_iter(
            columns
                .into_iter()
                .enumerate()
                .filter(|(_, pair)| {
                    let ok = !pair.sql.name.is_empty();
                    debug!("{pair:?} -> {ok}");
                    ok
                })
                .map(|(no, pair)| {
                    (
                        pair.sql.name.clone(),
                        ConstrainedField::new(&pair.title, &pair.sql.constraint, no),
                    )
                }),
        );

        let mut used_cols = Vec::<ConstrainedField>::with_capacity(code_col_dict.iter().count());
        for (_, values) in code_col_dict.iter_all() {
            for field in values {
                used_cols.push(field.clone());
            }
        }

        self.code_col_dict = code_col_dict;

        info!("{}: used_cols {:?}", function_path!(), self.code_col_dict);

        used_cols
    }

    #[named]
    fn init(&mut self) -> bool {
        trace!("{}", function_path!());
        self.first_row()
    }

    #[named]
    fn next(&mut self) -> bool {
        let mut work_id = 0;

        self.row_values.borrow_mut().clear();
        while self.next_row() {
            let wi = match self.read::<i64, _>("WorkId") {
                Ok(x) => x,
                Err(e) => panic!("{}", e),
            };
            if work_id == 0 {
                work_id = wi;
                if work_id < self.last_work_id as i64 {
                    self.last_work_id = 0_u64;
                    break;
                }
                self.row_values
                    .borrow_mut()
                    .insert("WorkId".to_string(), sqlite::Value::Integer(work_id));
                self.last_work_id = wi as u64;
            } else {
                if wi != work_id {
                    break;
                }
            }

            let code = match self.read::<ColName, _>("ColumnId") {
                Ok(x) => x,
                Err(e) => panic!("{}", e),
            };

            self.store_value(&code);
        }

        debug!(
            "{}: work_id {work_id} => {:?}",
            function_path!(),
            self.row_values
        );

        !self.row_values.borrow_mut().is_empty()
    }

    fn get_datetime(self: &mut SqlReader<'a>, id: &FldId) -> Option<DateTime<Utc>> {
        if id.is_empty() {
            return None;
        }

        if let Some(v) = self.get_value(id) {
            return match v {
                sqlite::Value::Binary(vec) => {
                    Some(get_date_time_from_filetime(u64::from_bytes(&vec)))
                }
                sqlite::Value::Null => None,
                _ => panic!("unexpected {v:?} for {id}"),
            };
        }

        None
    }

    fn get_int(&mut self, id: &FldId) -> Option<i64> {
        if id.is_empty() {
            return None;
        }

        if let Some(v) = self.get_value(id) {
            return match v {
                sqlite::Value::Integer(x) => Some(x),
                sqlite::Value::Binary(vec) => Some(i64::from_bytes(&vec)),
                sqlite::Value::Null => None,
                _ => panic!("unexpected {v:?} for {id}"),
            };
        }

        None
    }

    fn get_str(&mut self, id: &FldId) -> Option<String> {
        if id.is_empty() {
            return None;
        }

        if let Some(v) = self.get_value(id) {
            return match v {
                sqlite::Value::String(x) => Some(x),
                sqlite::Value::Null => None,
                _ => panic!("unexpected {v:?} for {id}"),
            };
        }

        None
    }

    fn get_guid(&mut self, id: &FldId) -> Option<String> {
        if let Some(s) = self.get_str(id) {
            return Some(find_guid(s.as_str(), (id.to_owned() + "=").as_str()));
        }
        None
    }
}

//--------------------------------------------------------------------
use csv::Writer;
use report::{Report, ReportJson};
use simple_error::SimpleError;
use std::path::Path;
//use std::process::id;
//use crate::ColumnType::String;

struct ReportCsv {
    writer: RefCell<Writer<File>>,
}

impl ReportCsv {
    pub fn new(report_path: &Path) -> Result<Self, SimpleError> {
        let writer = Writer::from_path(report_path.to_str().unwrap());
        let writer =
            writer.expect(format!("Could not create '{}'", report_path.display()).as_str());

        Ok(ReportCsv {
            writer: RefCell::new(writer),
        })
    }
}

impl Report for ReportCsv {
    #[named]
    fn set_field(&self, fld: &str) {
        trace!("{} {fld}", function_path!());
        let mut writer = self.writer.borrow_mut();
        writer.write_field(fld).unwrap();
    }

    #[named]
    fn new_record(&self) {
        trace!("{}", function_path!());
        let mut writer = self.writer.borrow_mut();
        writer.write_record(None::<&[u8]>).unwrap();
    }

    #[named]
    fn str_val(&self, _f: &str, s: String) {
        trace!("{} {_f}={s}", function_path!());
        let mut writer = self.writer.borrow_mut();
        writer.write_field(s.as_str()).unwrap();
    }

    #[named]
    fn int_val(&self, _f: &str, n: u64) {
        trace!("{} {_f}={n}", function_path!());
        let mut writer = self.writer.borrow_mut();
        writer.write_field(format!("{n}").as_str()).unwrap();
    }

    fn is_some_val_in_record(&self) -> bool {
        todo!()
    }
}

impl Drop for ReportCsv {
    fn drop(&mut self) {
        self.new_record();
    }
}

pub fn do_reports(cfg: &ReportsCfg, reader: &mut dyn FieldReader) {
    for report in &cfg.reports {
        do_report(report, reader, cfg.output_dir.as_str(), &cfg.output_format);
    }
}

#[named]
pub fn do_report(
    cfg: &ReportCfg,
    reader: &mut dyn FieldReader,
    output_dir: &str,
    output_format: &OutputFormat,
) {
    let mut out_path = std::path::PathBuf::from(output_dir);
    out_path.push(cfg.title.clone().replace(|c| "\\/ ".contains(c), "_"));

    info!(
        "{}: out_path: {out_path:?}, {output_format:?}",
        function_path!()
    );

    let reporter: Box<dyn Report> = match output_format {
        OutputFormat::Csv => {
            out_path.set_extension("csv");
            Box::new(ReportCsv::new(&out_path).unwrap())
        }
        OutputFormat::Json => {
            out_path.set_extension("json");
            Box::new(ReportJson::new(&out_path).unwrap())
        }
    };
    //println!("FileReport: {}", cfg.title);
    let used_cols = reader.get_used_columns(&cfg.columns);

    #[derive(Debug)]
    struct Column {
        title: String,
        kind: ColumnType,
        constraint: Option<String>,
        idx: usize,
    }
    let mut columns = Vec::<Column>::with_capacity(used_cols.len());

    used_cols.iter().for_each(|fld| {
        let title = &fld.name;
        let kind = cfg.columns.iter().find(|c| c.title == *title).unwrap().kind;

        columns.push(Column {
            title: title.clone(),
            kind,
            constraint: fld.constraint.clone(),
            idx: fld.idx,
        });
    });

    columns.sort_by_key(|fld| fld.idx);
    columns
        .iter()
        .for_each(|fld| reporter.set_field(&fld.title));

    const CONSTR_AUTO_FILL: &str = "auto_fill";
    const CONSTR_1_NOT_EMPTY: &str = "first_not_empty";
    const KNOWN_CONSTRS: [&str; 2] = [CONSTR_AUTO_FILL, CONSTR_1_NOT_EMPTY];

    let constrained_columns: HashMap<String, String> = columns
        .iter()
        .filter_map(|fld| {
            if fld.constraint.is_some() {
                Some((
                    fld.title.clone(),
                    fld.constraint.as_ref().unwrap().to_string(),
                ))
            } else {
                None
            }
        })
        .filter(|(_, constraint)| {
            KNOWN_CONSTRS
                .into_iter()
                .any(|constr| constraint.contains(constr))
        })
        .collect();

    let mut found_1_value =
        HashMap::<String, String>::with_capacity(constrained_columns.iter().count());

    constrained_columns.iter().for_each(|(col_id, _)| {
        assert!(reader.init());
        while reader.next() {
            if let Some(ref str) = reader.get_str(col_id) {
                if !str.is_empty() {
                    info!("first not empty {col_id} -> '{str}'");
                    found_1_value.insert(col_id.to_string(), str.clone());
                    break;
                }
            }
        }
    });

    info!("constrained_columns: {constrained_columns:?}");
    let mut auto_filled =
        HashMap::<String, String>::with_capacity(constrained_columns.iter().count());
    constrained_columns.iter().for_each(|(fld, value)| {
        if value.contains(CONSTR_AUTO_FILL) {
            info!("fld: '{fld}' -> {}", found_1_value[fld.as_str()]);
            auto_filled.insert(fld.to_string(), found_1_value[fld.as_str()].to_string());
        }
    });

    assert!(reader.init());

    while reader.next() {
        reporter.new_record();

        for col in &columns {
            let col_id = &col.title;

            //debug!("{col_id} constraint {:?}", col.constraint);
            if let Some(constraint) = &col.constraint {
                if !KNOWN_CONSTRS
                    .into_iter()
                    .any(|constr| constraint.contains(constr))
                {
                    if let Some(value) = reader.get_str(col_id) {
                        let expr = constraint.replace("{Value}", &value);

                        match evalexpr::eval_boolean(&expr) {
                            Ok(ok) => {
                                if !ok {
                                    debug!("skip {col_id}='{value}' due constraint '{expr}'");
                                    reporter.str_val(col.title.as_str(), "".to_string());
                                    continue;
                                }
                            }
                            Err(e) => panic!("Eval constraint failed: {e}"),
                        };
                    }
                }
            }

            match col.kind {
                ColumnType::String => {
                    let s = if let Some(str) = reader.get_str(col_id) {
                        str
                    } else {
                        if auto_filled.contains_key(col_id) {
                            auto_filled[col_id].clone()
                        } else {
                            "".to_string()
                        }
                    };
                    reporter.str_val(col.title.as_str(), s);
                }
                ColumnType::Integer => {
                    if let Some(v) = reader.get_int(col_id) {
                        reporter.int_val(col.title.as_str(), v as u64);
                    } else {
                        reporter.str_val(col.title.as_str(), "".to_string());
                    }
                }
                ColumnType::DateTime => {
                    if let Some(dt) = reader.get_datetime(col_id) {
                        reporter.str_val(col.title.as_str(), utils::format_date_time(dt));
                    } else {
                        reporter.str_val(col.title.as_str(), "".to_string());
                    }
                }
                ColumnType::GUID => {
                    if let Some(guid) = reader.get_guid(col_id) {
                        reporter.str_val(col.title.as_str(), guid);
                    } else {
                        reporter.str_val(col.title.as_str(), "".to_string());
                    }
                }
            }
        }
    }

    reporter.footer();
}
