#![allow(non_upper_case_globals)]
#[warn(non_camel_case_types)]
pub mod report;
#[allow(non_camel_case_types)]
pub mod utils;

use crate::utils::column_string_part;
use ::function_name::named;
use log::{debug, error, info, trace};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub constraint: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum OutputType {
    ToFile,
    ToStdout,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportCfg {
    pub title: String,
    pub output_filename: String,
    pub constraint: Option<String>,
    pub columns: Vec<ColumnPair>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportsCfg {
    pub table_edb: String,
    pub table_sql: String,
    pub output_format: OutputFormat,
    pub output_type: OutputType,
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
    hidden: bool,
    optional: bool,
    idx: usize,
}

impl ConstrainedField {
    fn new(name: &str, constraints: &Option<Vec<String>>, idx: usize) -> Self {
        let mut hidden = false;
        let mut optional = false;
        let mut constraint = None;

        if let Some(constraints) = constraints {
            for s in constraints {
                match s.as_str() {
                    CONSTR_HIDDEN => hidden = true,
                    CONSTR_OPTIONAL => optional = true,
                    _ => constraint = Some(s.clone()),
                }
            }
        }

        Self {
            name: name.to_string(),
            constraint,
            hidden,
            optional,
            idx,
        }
    }
}

pub trait FieldReader {
    fn get_used_columns(&mut self, columns: &[ColumnPair]) -> Vec<ConstrainedField>;
    fn init(&mut self) -> bool;
    fn next(&mut self) -> bool;
    fn get_int(&mut self, id: &FldId) -> Option<i64>;
    fn get_str(&mut self, id: &FldId) -> Option<String>;
    fn get_guid(&mut self, id: &FldId) -> Option<String>;
    fn get_datetime(&mut self, id: &FldId) -> Option<DateTime<Utc>>;
}

//--------------------------------------------------------------------
use ese_parser_lib::vartime::{get_date_time_from_filetime, VariantTimeToSystemTime, SYSTEMTIME};
use ese_parser_lib::{ese_parser::EseParser, ese_trait::*, DbState};

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
    pub jdb: Box<EseParser<BufReader<File>>>,
    filename: String,
    table: u64,
    tablename: String,
    col_infos: HashMap<String, (u32, u32)>,
    rec_no: u64,
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
        Err(_e) => panic!("Error: {_e}"),
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
            rec_no: 0,
        }
    }
}

impl FieldReader for EseReader {
    #[named]
    fn get_used_columns(&mut self, columns: &[ColumnPair]) -> Vec<ConstrainedField> {
        trace!("{}", function_path!());
        let mut used_cols = Vec::<ConstrainedField>::with_capacity(columns.len());
        let tablename = &self.tablename;
        let cols = self.jdb.get_columns(tablename).unwrap();
        let col_infos = &mut self.col_infos;
        let mut idx = 0_usize;
        for col_pair in columns {
            let name = col_pair.edb.name.clone();

            if !name.is_empty() {
                match cols
                    .iter()
                    .find(|col| col.name == name || column_string_part(&col.name) == name)
                {
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

        used_cols
    }

    #[named]
    fn init(&mut self) -> bool {
        trace!("{}", function_path!());
        self.rec_no = 0;
        self.jdb.move_row(self.table, ESE_MoveFirst).unwrap()
    }

    //#[named]
    fn next(&mut self) -> bool {
        //trace!("{}", function_path!());
        let ok = if self.rec_no > 0 {
            self.jdb.move_row(self.table, ESE_MoveNext).unwrap()
        } else {
            true
        };
        self.rec_no += 1;
        ok
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
            Ok(r) => r.map(|v| from_utf16(v.as_slice())),
            Err(_e) => panic!("{id} - error: {_e}"),
        }
    }

    fn get_guid(&mut self, id: &FldId) -> Option<String> {
        if let Some(s) = self.get_str(id) {
            return Some(find_guid(s.as_str(), (id.to_owned() + "=").as_str()));
        }
        None
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
    pub fn new(db_path: &str) -> Self {
        let conn = Connection::open_with_flags(db_path, OpenFlags::new().set_read_only()).unwrap();
        let sql = "select WorkId, * from SystemIndex_1_PropertyStore order by WorkId";
        let session = Session::new_with_fn(Box::new(conn), unsafe {
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
            //debug!("store_value: skip code '{code}'");
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
    fn get_used_columns(&mut self, columns: &[ColumnPair]) -> Vec<ConstrainedField> {
        trace!("{}", function_path!());

        let code_col_dict: CodeColDict = CodeColDict::from_iter(
            columns
                .iter()
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

        code_col_dict
            .flat_iter()
            .for_each(|(k, v)| self.code_col_dict.insert(k.clone(), v.clone()));

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
            } else if wi != work_id {
                break;
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
use crate::report::{ReportFormat, ReportOutput, ReportProducer};
use evalexpr::{Context, ContextWithMutableVariables, IterateVariablesContext, Value};
use report::Report;
use std::path::Path;

#[derive(Debug)]
struct ReportColumn {
    title: String,
    kind: ColumnType,
    constraint: Option<String>,
    hidden: bool,
    optional: bool,
    _idx: usize,
}

impl<R: Report + ?Sized> Report for Box<R> {
    fn create_new_row(&mut self) {
        (**self).create_new_row()
    }

    fn insert_str_val(&self, f: &str, s: String) {
        (**self).insert_str_val(f, s)
    }

    fn insert_int_val(&self, f: &str, n: u64) {
        (**self).insert_int_val(f, n)
    }

    fn is_some_val_in_record(&self) -> bool {
        (**self).is_some_val_in_record()
    }
}

//#[named]
pub fn do_reports(
    cfg: &ReportsCfg,
    reader: &mut dyn FieldReader,
    edb_database_state: Option<DbState>,
) {
    //println!("FileReport: {}", cfg.title);
    struct ReportDef {
        title: String,
        reporter: Box<dyn Report>,
        columns: Vec<ReportColumn>,
        constrain: Option<evalexpr::Node>,
        constrained_columns: HashMap<String, String>,
        auto_filled: HashMap<String, String>,
    }
    let mut reports: Vec<ReportDef> = Vec::new();

    let report_format = match cfg.output_format {
        OutputFormat::Csv => ReportFormat::Csv,
        OutputFormat::Json => ReportFormat::Json,
    };

    let report_type = match cfg.output_type {
        OutputType::ToStdout => ReportOutput::ToStdout,
        OutputType::ToFile => ReportOutput::ToFile,
    };

    let rep_factory = ReportProducer::new(cfg.output_dir.as_ref(), report_format, report_type);
    let mut cached = HashMap::<String, String>::new();

    for report in &cfg.reports {
        let output_filename_title = &report.output_filename;
        let mut output_filename = "".to_string();

        if cached.contains_key(output_filename_title) {
            output_filename = cached[output_filename_title].clone();
        } else {
            if output_filename_title == "System_ComputerName" {
                // ASDF-5849
                // special case for System_ComputerName
                // read last value and if System_ItemType != ".url"

                let system_itemtype = "System_ItemType".to_string();

                let col_for_computername = report
                    .columns
                    .iter()
                    .find(|col| col.title == *output_filename_title)
                    .unwrap_or_else(|| {
                        panic!("No column for output_filename '{output_filename_title}'")
                    });

                let col_for_itemtype = ColumnPair {
                    title: system_itemtype.clone(),
                    kind: ColumnType::String,
                    edb: Column {
                        name: system_itemtype.clone(),
                        constraint: None,
                    },
                    sql: Column {
                        name: "567".to_string(),
                        constraint: None,
                    },
                };

                let _columns = reader
                    .get_used_columns(&[(*col_for_computername).clone(), col_for_itemtype.clone()]);

                if !reader.init() {
                    panic!("reader.init() failed");
                }

                while reader.next() {
                    if let Some(ref str) = reader.get_str(output_filename_title) {
                        if !str.is_empty() {
                            if let Some(ref item_type) = reader.get_str(&system_itemtype) {
                                if !item_type.is_empty() && item_type == ".url" {
                                    // skip
                                    continue;
                                }
                            }

                            output_filename = str.clone();
                        }
                    }
                }
            } else {
                // get first non-empty result

                let col_for_filename = report
                    .columns
                    .iter()
                    .find(|col| col.title == *output_filename_title)
                    .unwrap_or_else(|| {
                        panic!("No column for output_filename '{output_filename_title}'")
                    });
                let _columns = reader.get_used_columns(&[(*col_for_filename).clone()]);

                if !reader.init() {
                    panic!("reader.init() failed");
                }

                while reader.next() {
                    if let Some(ref str) = reader.get_str(output_filename_title) {
                        if !str.is_empty() {
                            output_filename = str.clone();
                            info!(
                                "output_filename '{output_filename_title}' -> '{output_filename}'"
                            );
                            break;
                        }
                    }
                }
            }

            info!("output_filename '{output_filename_title}' -> '{output_filename}'");

            cached.insert(
                output_filename_title.to_string(),
                output_filename.to_string(),
            );
        }
        let (_out_path, reporter) = rep_factory
            .new_report(
                Path::new(""),
                &output_filename,
                &report.title,
                edb_database_state,
            )
            .unwrap();

        let columns = get_used_columns(report, reader, &*reporter);
        info!("{} columns: {columns:?}", report.title);

        let constrained_columns = get_constrained_cols(&columns);
        info!("constrained_columns: {constrained_columns:?}");
        let auto_filled = get_autofilled_cols(
            &constrained_columns,
            &HashMap::from([(
                output_filename_title.to_string(),
                output_filename.to_string(),
            )]),
        );

        reports.push(ReportDef {
            reporter,
            columns,
            constrained_columns,
            auto_filled,
            title: report.title.clone(),
            constrain: if let Some(ref expr) = report.constraint {
                match evalexpr::build_operator_tree(expr) {
                    Ok(node) => Some(node),
                    Err(e) => panic!("failed parsing of '{expr}': {e}"),
                }
            } else {
                None
            },
        });
    }

    let mut context = evalexpr::HashMapContext::new();
    if !reader.init() {
        panic!("reader.init() failed");
    }

    while reader.next() {
        reports.iter().for_each(|r| {
            debug!("flag {} -> false", r.title);
            context
                .set_value(r.title.clone(), Value::Boolean(false))
                .unwrap()
        });

        'report: for report in &mut reports {
            if let Some(ref constr) = report.constrain {
                match constr.eval_with_context_mut(&mut context) {
                    Ok(ok) => {
                        if let Value::Boolean(ok) = ok {
                            if !ok {
                                debug!(
                                    "skip report '{}' due constraint '{}'",
                                    report.title, constr
                                );
                                context.iter_variable_names().for_each(|nm| {
                                    debug!("  {}: {:?}", nm, context.get_value(&nm))
                                });
                                continue 'report;
                            };
                        }
                    }
                    Err(e) => panic!(
                        "failed evaluation of '{}' for report {}: {e}",
                        constr, report.title
                    ),
                }
            }

            for (col_id, constraint) in &report.constrained_columns {
                if VALIDATED_CONSTRS
                    .into_iter()
                    .any(|constr| constraint.contains(constr))
                {
                    //debug!("{col_id} constraint {constraint:?}");

                    if let Some(value) = reader.get_str(col_id) {
                        if value.is_empty() {
                            debug!("skip empty '{col_id}' with constraint in {}", report.title);
                            continue 'report;
                        }

                        let value = value.as_bytes().escape_ascii().to_string();
                        let expr = constraint.replace("{Value}", &value);
                        debug!("{col_id} testing {expr:?}");

                        match evalexpr::eval_boolean(&expr) {
                            Ok(ok) => {
                                if !ok {
                                    debug!(
                                        "skip {col_id}='{value}' due constraint '{expr}' in {}",
                                        report.title
                                    );
                                    continue 'report;
                                }
                            }
                            Err(e) => error!("Eval constraint '{expr}' failed: {e}"),
                        };
                    } else {
                        let col = report.columns.iter().find(|c| c.title == *col_id).unwrap();
                        if !col.optional {
                            debug!("skip None '{col_id}' with constraint in {}", report.title);
                            continue 'report;
                        }
                    }
                }
            }

            report.reporter.create_new_row();
            debug!("flag {} -> true", report.title);
            context
                .set_value(report.title.clone(), Value::Boolean(true))
                .unwrap();

            for col in &report.columns {
                if col.hidden {
                    continue;
                }

                let col_id = &col.title;

                match col.kind {
                    ColumnType::String => {
                        // let s = if let Some(str) = reader.get_str(col_id) {
                        //     str
                        // } else {
                        //     if report.auto_filled.contains_key(col_id) {
                        //         report.auto_filled[col_id].clone()
                        //     } else {
                        //         "".to_string()
                        //     }
                        // };

                        let s = if report.auto_filled.contains_key(col_id) {
                            report.auto_filled[col_id].clone()
                        } else if let Some(str) = reader.get_str(col_id) {
                            str
                        } else {
                            "".to_string()
                        };
                        if !s.is_empty() {
                            report.reporter.insert_str_val(col.title.as_str(), s);
                        }
                    }
                    ColumnType::Integer => {
                        if let Some(v) = reader.get_int(col_id) {
                            report.reporter.insert_int_val(col.title.as_str(), v as u64);
                        }
                    }
                    ColumnType::DateTime => {
                        if let Some(dt) = reader.get_datetime(col_id) {
                            report
                                .reporter
                                .insert_str_val(col.title.as_str(), utils::format_date_time(dt));
                        }
                    }
                    ColumnType::GUID => {
                        if let Some(guid) = reader.get_guid(col_id) {
                            report.reporter.insert_str_val(col.title.as_str(), guid);
                        }
                    }
                }
            }

            report.reporter.footer();
        }
    }
}

fn get_autofilled_cols(
    constrained_columns: &HashMap<String, String>,
    found_1_value: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut auto_filled =
        HashMap::<String, String>::with_capacity(constrained_columns.iter().count());
    constrained_columns.iter().for_each(|(fld, value)| {
        if value.contains(CONSTR_AUTO_FILL) {
            info!("fld: '{fld}' -> {}", found_1_value[fld.as_str()]);
            auto_filled.insert(fld.to_string(), found_1_value[fld.as_str()].to_string());
        }
    });
    auto_filled
}

const CONSTR_AUTO_FILL: &str = "auto_fill";
const CONSTR_HIDDEN: &str = "hidden";
const CONSTR_OPTIONAL: &str = "optional";
const CONSTR_REGEX: &str = "regex_matches";
const KNOWN_CONSTRS: [&str; 4] = [
    CONSTR_AUTO_FILL,
    CONSTR_HIDDEN,
    CONSTR_REGEX,
    CONSTR_OPTIONAL,
];
const VALIDATED_CONSTRS: [&str; 1] = [CONSTR_REGEX];

fn get_constrained_cols(columns: &[ReportColumn]) -> HashMap<String, String> {
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
    constrained_columns
}

fn get_used_columns(
    cfg: &ReportCfg,
    reader: &mut dyn FieldReader,
    reporter: &dyn Report,
) -> Vec<ReportColumn> {
    let used_cols = reader.get_used_columns(&cfg.columns);

    let mut columns = Vec::<ReportColumn>::with_capacity(used_cols.len());

    used_cols.iter().for_each(|fld| {
        let title = &fld.name;
        let kind = cfg.columns.iter().find(|c| c.title == *title).unwrap().kind;

        columns.push(ReportColumn {
            title: title.clone(),
            kind,
            constraint: fld.constraint.clone(),
            hidden: fld.hidden,
            optional: fld.optional,
            _idx: fld.idx,
        });
    });

    // call set_field for all fields used in cfg (even empty one)
    cfg.columns.iter().for_each(|cc| {
        let hidden = columns.iter().any(|c| c.title == cc.title && c.hidden);
        if !hidden {
            debug!("set header '{}' for '{}' ", cc.title, cfg.title);
            reporter.set_field(&cc.title);
        }
    });

    columns
}
