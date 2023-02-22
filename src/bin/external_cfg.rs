use serde::{Deserialize, Serialize};
use serde_yaml;

#[derive(Debug, Serialize, Deserialize)]
enum ColumnType {
    String,
    Integer,
    DateTime
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
struct FileReportCfg {
    title: String,
    table_edb: String,
    table_sql: String,
    columns: Vec<ColumnPair>
}

//--------------------------------------------------------------------
use chrono::{DateTime, Utc, NaiveDateTime};

type FldId = String;

trait FieldReader {
    fn next(&mut self) -> bool;
    fn get_int(&mut self, id: &FldId) -> Option<i64>;
    fn get_str(&mut self, id: &FldId) -> Option<String>;
    fn get_datetime(&mut self, id: &FldId) -> Option<DateTime<Utc>>;
}

//--------------------------------------------------------------------
use std::rc::Rc;
use ouroboros::self_referencing;
use sqlite;

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

                if let Some(naive_datetime) = NaiveDateTime::from_timestamp_opt(nanos / A_BILLION, (nanos % A_BILLION) as u32) {
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

fn do_report(cfg: &FileReportCfg, reader: &mut dyn FieldReader) {

    println!("FileReport: {}", cfg.title);
    while reader.next() {
        for col in &cfg.columns {
            print!("  {} -> ", col.title);
            let col_id = &col.title;

            match col.kind {
                ColumnType::String => {
                    let s = if let Some(str) = reader.get_str(col_id) 
                                        {
                                            str
                                        } else {
                                            "".to_string()
                                        };
                    println!("{}", s);
                }
                ColumnType::Integer => {
                    let s = if let Some(v) = reader.get_int(col_id) {v} else {0};
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

fn main() {
    let cli = Cli::parse();
    let s = fs::read_to_string(cli.cfg).unwrap();
    let cfg: FileReportCfg = serde_yaml::from_str(s.as_str()).unwrap();

    if let Some(db_path) = cli.sql {
        let connection = Rc::new(sqlite::Connection::open(db_path).unwrap());
        let mut fields = Vec::<&'static str>::new();

        for col_pair in &cfg.columns {
            let code = &col_pair.sql.name;

            if !code.is_empty() {
                let name = &col_pair.title;
                let sql = format!("CREATE TEMP VIEW {name} AS SELECT WorkId, Value as {name} from {table} where ColumnId = {code};", 
                                            table = cfg.table_sql);
                connection.execute(sql.as_str()).expect(format!("bad sql: '{sql}'").as_str());
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
    } else {
        println!("missed `--sql` argument");
    }
}
