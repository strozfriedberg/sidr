#![allow(non_upper_case_globals)]
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{collections::HashMap, string::String};

#[derive(Debug, Serialize, Deserialize)]
enum ColumnType {
    String,
    Integer,
    DateTime,
    GUID
}

#[derive(Debug, Serialize, Deserialize)]
struct Column {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ColumnPair {
    title: String,
    kind: ColumnType,
    edb:  Column,
    sql:  Column,
}

#[derive(Debug, Serialize, Deserialize)]
enum OutputFormat {
    Csv,
    Json,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReportCfg {
    title: String,
    columns: Vec<ColumnPair>
}

#[derive(Debug, Serialize, Deserialize)]
struct ReportsCfg {
    table_edb: String,
    table_sql: String,
    output_format: OutputFormat,
    output_dir: String,
    reports: Vec<ReportCfg>
}

//--------------------------------------------------------------------
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

type FldId = String;

trait FieldReader {
    fn init(&mut self, columns: &Vec<ColumnPair>);
    fn next(&mut self) -> bool;
    fn get_int(&mut self, id: &FldId) -> Option<i64>;
    fn get_str(&mut self, id: &FldId) -> Option<String>;
    fn get_guid(&mut self, id: &FldId) -> Option<String>;
    fn get_datetime(&mut self, id: &FldId) -> Option<DateTime<Utc>>;
}

//--------------------------------------------------------------------
use ese_parser_lib::{ese_parser::EseParser, ese_trait::*};
use ese_parser_lib::vartime::{get_date_time_from_filetime, SYSTEMTIME, VariantTimeToSystemTime};
use std::{fs::File, io::BufReader, str};
use num;

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

struct EseReader {
    jdb: Box<EseParser<BufReader<File>>>,
    filename: String,
    table: u64,
    tablename: String,
    col_infos: HashMap<String, (u32, u32)>,
}

fn get_column<T: FromBytes + num::NumCast>(jdb: &dyn EseDb, table: u64, column: u32) -> Option<i64> {
    match jdb.get_column(table, column) {
        Ok(r) => match r {
            Some(v) => num::cast::<_, i64>(T::from_bytes(&v)),
            None => None,
        },
        Err(e) => panic!("Error: {e}"),
    }
}

impl EseReader {
    fn new(filename: &str, tablename: &str) -> Self {
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
    fn init(&mut self, columns: &Vec<ColumnPair>) {
        let tablename= &self.tablename;
        let cols = self.jdb.get_columns(tablename).unwrap();
        let col_infos = &mut self.col_infos;
        for col_pair in columns {
            let name = col_pair.edb.name.clone();

            if !name.is_empty() {
                match cols.iter().find(|col| col.name == name) {
                    Some(col_info) => {
                        col_infos.insert(
                            col_pair.title.clone(),
                            (col_info.id, field_size(col_info.typ, col_info.cbmax)),
                        );
                    }
                    None => panic!("Could not find '{name}' column in '{tablename}' table in '{}'", self.filename),
                }
            }
        }
    }

    fn next(&mut self) -> bool {
        self.jdb.move_row(self.table, ESE_MoveNext).unwrap()
    }

    fn get_datetime(&mut self, id: &FldId) -> Option<DateTime<Utc>> {
        if !self.col_infos.contains_key(id) {
            return None;
        }
        // match self.jdb.get_column_date(self.table, self.col_infos[id].0) {
        //     Ok(date) => date,
        //     Err(e) => panic!("Error '{}'", e.as_str()),
        // }

        let r = self.jdb.get_column(self.table, self.col_infos[id].0).unwrap();
        if let Some(v) = r {
            if let Ok(val) = v.clone().try_into() {
                let vartime = f64::from_le_bytes(val);
                let mut st = SYSTEMTIME::default();
                if VariantTimeToSystemTime(vartime, &mut st) {
                    let datetime = Utc
                        .with_ymd_and_hms(st.wYear as i32, st.wMonth as u32, st.wDay as u32,
                                          st.wHour as u32, st.wMinute as u32, st.wSecond as u32).single().unwrap(); // this is obviously not the right function! I didn't know what the right one was off the top of my head. We need to include the time component. also needs to be something that returns a DateTime.
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
            _ => panic!("{fld_size} - wrong size of int field"),
        }
    }

    fn get_str(&mut self, id: &FldId) -> Option<String> {
        if !self.col_infos.contains_key(id) {
            return None;
        }
        match self.jdb.get_column(self.table, self.col_infos[id].0) {
            Ok(r) => match r {
                Some(v) =>
                match str::from_utf8(v.as_slice()) {
                    Ok(s) => Some(s.to_string()),
                    Err(e) => panic!("Invalid UTF-8 sequence: {e}"),
                },
                None => None,
            },
            Err(e) => panic!("Error: {e}"),
        }
    }

    fn get_guid(&mut self, id: &FldId) -> Option<String> {
        todo!()
    }
}

//--------------------------------------------------------------------
use ouroboros::self_referencing;
use sqlite;
use std::rc::Rc;

#[self_referencing]
struct SqlReader {
    connection: Rc<sqlite::Connection>,
    #[borrows(mut connection)]
    #[not_covariant]
    statement: sqlite::Statement<'this>,
}

impl FieldReader for SqlReader {
    fn init(&mut self, columns: &Vec<ColumnPair>) {
        todo!()
    }

    fn next(&mut self) -> bool {
        self.with_statement_mut(|st| st.next().is_ok())
    }

    fn get_datetime(self: &mut SqlReader, id: &FldId) -> Option<DateTime<Utc>> {
        if id.is_empty() {
            return None;
        }
        if let Ok(vec) = self.with_statement_mut(|st| st.read::<Vec<u8>, _>(id.as_str())) {
            if let Ok(bytes) = vec.try_into() {
                let nanos = i64::from_le_bytes(bytes);
                const A_BILLION: i64 = 1_000_000_000;

                if let Some(naive_datetime) =
                    NaiveDateTime::from_timestamp_opt(nanos / A_BILLION, (nanos % A_BILLION) as u32)
                {
                    return Some(DateTime::<Utc>::from_utc(naive_datetime, Utc));
                }
            }
        }
        None
    }

    fn get_int(&mut self, id: &FldId) -> Option<i64> {
        if id.is_empty() {
            return None;
        }
        match self.with_statement_mut(|st| st.read::<i64, _>(id.as_str())) {
            Ok(x) => Some(x),
            Err(e) => panic!("{e}"),
        }
    }

    fn get_str(&mut self, id: &FldId) -> Option<String> {
        if id.is_empty() {
            return None;
        }
        match self.with_statement_mut(|st| st.read::<String, _>(id.as_str())) {
            Ok(x) => Some(x),
            Err(e) => panic!("{e}"),
        }
    }

    fn get_guid(&mut self, id: &FldId) -> Option<String> {
        todo!()
    }
}

//--------------------------------------------------------------------
use std::fs;
#[path="../report.rs"]
mod report;
use crate::report::*;

fn do_reports(cfg: &ReportsCfg, reader: &mut dyn FieldReader) {
    for report in &cfg.reports {
        do_report(report, reader, cfg.output_dir.as_str(), &cfg.output_format);
    }
}

fn do_report(cfg: &ReportCfg, reader: &mut dyn FieldReader, output_dir: &str, output_format: &OutputFormat) {
    let mut out_path = PathBuf::from(output_dir);
    out_path.push(cfg.title.clone());

    let reporter: Box<dyn Report> = match output_format {
        OutputFormat::Csv => {
            out_path.set_extension("csv");
            Box::new(ReportCsv::new(&out_path).unwrap())
        },
        OutputFormat::Json => {
            out_path.set_extension("json");
            Box::new(ReportJson::new(&out_path).unwrap())
        }
    };
    //println!("FileReport: {}", cfg.title);
    reader.init(&cfg.columns);

    while reader.next() {
        reporter.new_record();

        for col in &cfg.columns {
            let col_id = &col.title;

            match col.kind {
                ColumnType::String => {
                    let s = if let Some(str) = reader.get_str(col_id) {
                        str
                    } else {
                        "".to_string()
                    };
                    reporter.str_val(col.title.as_str(), s);
                }
                ColumnType::Integer => {
                    if let Some(v) = reader.get_int(col_id) {
                        reporter.int_val(col.title.as_str(), v as u64);
                    };
                }
                ColumnType::DateTime => {
                    if let Some(dt) = reader.get_datetime(col_id) {
                        reporter.str_val(col.title.as_str(), format!("{dt}"));
                    }
                }
                ColumnType::GUID => {
                    if let Some(dt) = reader.get_guid(col_id) {
                        reporter.str_val(col.title.as_str(), format!("{dt}"));
                    }
                }
            }
        }
    }

    reporter.footer();
}

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    /// Path to <config.yaml>
    #[arg(short, long)]
    cfg_path: String,
    /// Path to the directory where reports will be created (will be created if not present).
    /// Default is the current directory.
    #[arg(short, long)]
    outdir: Option<String>,
    /// json (default) or csv.
    #[arg(short, long)]
    format: Option<String>,
    /// Path to SQL/EDB database
    #[arg(short, long)]
    db_path: PathBuf,
}

fn do_sql_report(db_path: &str, cfg: &ReportsCfg) {
    todo!()
/*    let connection = Rc::new(sqlite::Connection::open(db_path).unwrap());
    let mut fields = Vec::<&'static str>::new();

    for col_pair in &cfg.columns {
        let code = &col_pair.sql.name;

        if !code.is_empty() {
            let name = &col_pair.title;
            let sql = format!("CREATE TEMP VIEW {name} AS SELECT WorkId, Value as {name} from {table} where ColumnId = {code};",
                              table = cfg.table_sql);
            connection
                .execute(sql.as_str())
                .unwrap_or_else(|_| panic!("bad sql: '{sql}'"));
            fields.push(name.as_str());
        }
    }

    let mut select = "SELECT ".to_string();
    select.push_str(fields.join(",").as_str());
    select = format!("{} from {} as a", select, fields[0]);
    for field in &mut fields[1..] {
        select.push_str(format!(" LEFT JOIN {field} on a.WorkId = {field}.WorkId").as_str());
    }

    let mut sql_reader = SqlReaderBuilder {
        connection: connection,
        statement_builder: |connection| connection.prepare(select).unwrap(),
    }
    .build();

    do_reports(cfg, &mut sql_reader);
*/
}

fn do_edb_report(db_path: &str, cfg: &ReportsCfg) {
    let mut edb_reader = EseReader::new(db_path, &cfg.table_edb);
    do_reports(cfg, &mut edb_reader);
}

fn main() {
    let cli = Cli::parse();
    let s = fs::read_to_string(&cli.cfg_path).unwrap();
    let mut cfg: ReportsCfg = serde_yaml::from_str(s.as_str()).unwrap();
    let db_path = cli.db_path.display().to_string();

    if let Some(output_dir) = &cli.outdir {
        cfg.output_dir = output_dir.clone();
    }

    if let Some(output_format) = &cli.outdir {
        cfg.output_format = match output_format.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "csv" => OutputFormat::Csv,
            _ => panic!("Unknow output format '{output_format}'"),
        }
    }

    if db_path.ends_with("Windows.edb") {
        do_edb_report(db_path.as_str(), &cfg);
    } else if db_path.ends_with("Windows.db") {
        do_sql_report(db_path.as_str(), &cfg);
    }
}
