#![allow(non_upper_case_globals)]
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{collections::HashMap, string::String};

#[derive(Debug, Serialize, Deserialize)]
enum ColumnType {
    String,
    Integer,
    DateTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct Column {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ColumnPair {
    title: String,
    kind: ColumnType,
    edb: Column,
    sql: Column,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileReportCfg {
    title: String,
    table_edb: String,
    table_sql: String,
    columns: Vec<ColumnPair>,
}

//--------------------------------------------------------------------
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

type FldId = String;

trait FieldReader {
    fn next(&mut self) -> bool;
    fn get_int(&mut self, id: &FldId) -> Option<i64>;
    fn get_str(&mut self, id: &FldId) -> Option<String>;
    fn get_datetime(&mut self, id: &FldId) -> Option<DateTime<Utc>>;
}

//--------------------------------------------------------------------
use ese_parser_lib::{ese_parser::EseParser, ese_trait::*};
use ese_parser_lib::vartime::{get_date_time_from_filetime, SYSTEMTIME, VariantTimeToSystemTime};
use std::{fs::File, io::BufReader, str};
use num;

struct EseReader {
    jdb: Box<EseParser<BufReader<File>>>,
    table: u64,
    col_infos: HashMap<String, (u32, u32)>,
}

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
        _ => panic!("{} - unknown field type", col_type),
    }
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
    fn new(filename: &str, tablename: &str, columns: &Vec<ColumnPair>) -> Self {
        let jdb = Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, filename).unwrap());
        let table = jdb.open_table(tablename).unwrap();
        let cols = jdb.get_columns(tablename).unwrap();
        let mut col_infos = HashMap::<String, (u32, u32)>::new();
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
                    None => panic!(
                        "Could not find '{}' column in '{}' table in '{}'",
                        name, tablename, filename
                    ),
                }
            }
        }

        EseReader {
            jdb,
            table,
            col_infos,
        }
    }
}

impl FieldReader for EseReader {
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
                if VariantTimeToSystemTime(vartime as f64, &mut st) {
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
            _ => panic!("{} - wrong size of int field", fld_size),
        }
    }

    fn get_str(&mut self, id: &FldId) -> Option<String> {
        if !self.col_infos.contains_key(id) {
            return None;
        }
        match self.jdb.get_column(self.table, self.col_infos[id].0) {
            Ok(r) => match r {
                Some(v) =>
                match str::from_utf8(&v.as_slice()) {
                    Ok(s) => Some(s.to_string()),
                    Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                },
                None => None,
            },
            Err(e) => panic!("Error: {e}"),
        }
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
}

//--------------------------------------------------------------------
use std::fs;
//use std::path::Path;

fn do_report(cfg: &FileReportCfg, reader: &mut dyn FieldReader) {
    println!("FileReport: {}", cfg.title);
    while reader.next() {
        for col in &cfg.columns {
            print!("  {} -> ", col.title);
            let col_id = &col.title;

            match col.kind {
                ColumnType::String => {
                    let s = if let Some(str) = reader.get_str(col_id) {
                        str
                    } else {
                        "".to_string()
                    };
                    println!("{}", s);
                }
                ColumnType::Integer => {
                    let s = if let Some(v) = reader.get_int(col_id) {
                        v
                    } else {
                        0
                    };
                    println!("{}", s);
                }
                ColumnType::DateTime => {
                    if let Some(dt) = reader.get_datetime(col_id) {
                        println!("{}", dt);
                    } else {
                        println!();
                    }
                }
            }
        }
    }
}

use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// Path to <config.yaml>
    #[arg(short, long)]
    cfg: String,
    /// Path to EDB database
    #[arg(short, long)]
    edb: Option<String>,
    /// Path to SQL database
    #[arg(short, long)]
    sql: Option<String>,
}

fn do_sql_report(db_path: &String, cfg: &FileReportCfg) {
    let connection = Rc::new(sqlite::Connection::open(db_path).unwrap());
    let mut fields = Vec::<&'static str>::new();

    for col_pair in &cfg.columns {
        let code = &col_pair.sql.name;

        if !code.is_empty() {
            let name = &col_pair.title;
            let sql = format!("CREATE TEMP VIEW {name} AS SELECT WorkId, Value as {name} from {table} where ColumnId = {code};",
                              table = cfg.table_sql);
            connection
                .execute(sql.as_str())
                .expect(format!("bad sql: '{sql}'").as_str());
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

    do_report(&cfg, &mut sql_reader);
}

fn do_edb_report(db_path: &String, cfg: &FileReportCfg) {
    let mut edb_reader = EseReader::new(&db_path, &cfg.table_edb, &cfg.columns);
    do_report(&cfg, &mut edb_reader);
}

fn main() {
    let cli = Cli::parse();
    let s = fs::read_to_string(&cli.cfg).unwrap();
    let cfg: FileReportCfg = serde_yaml::from_str(s.as_str()).unwrap();

    if let Some(db_path) = &cli.edb {
        do_edb_report(db_path, &cfg);
    } else {
        println!("missed `--edb` argument");
    }

    if let Some(db_path) = &cli.sql {
        do_sql_report(db_path, &cfg);
    } else {
        println!("missed `--sql` argument");
    }
}
