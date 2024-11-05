use itertools::Itertools;
use simple_error::SimpleError;
use std::collections::HashMap;
use std::path::Path;

use crate::report::*;
use crate::shared::*;
use crate::utils::*;

use ese_parser_lib::ese_parser::FromBytes;
use sqlite::State;
use std::io::Write;

macro_rules! map_err(($result:expr) => ($result.map_err(|e| SimpleError::new(format!("{}", e)))));

fn sqlite_get_hostname(c: &sqlite::Connection) -> Result<String, SimpleError> {
    // We take the System.ComputerName field from each record, filter out any records
    // where the System.ItemType field is equal to ".url", and save the first one as the computer
    // name for the entire report.
    let q = "select WorkId as wId, Value
             from SystemIndex_1_PropertyStore_Metadata
             join SystemIndex_1_PropertyStore
             on Id = ColumnId
             where Name == 'System.ComputerName'
             and (
                 select Value
                 from SystemIndex_1_PropertyStore_Metadata
                 join SystemIndex_1_PropertyStore
                 on Id = ColumnId
                 where WorkId == wId
                 and Name == 'System.ItemType'
                 ) <> '.url' limit 1;"
        .to_string();
    let mut s = map_err!(c.prepare(q))?;
    if let Ok(State::Row) = s.next() {
        let val = map_err!(s.read::<Vec<u8>, _>("Value"))?;
        return Ok(String::from_utf8_lossy(&val).into_owned());
    }
    Err(SimpleError::new(
        "Empty field System.ComputerName".to_string(),
    ))
}

fn populate_property_id_maps<'a>(
    c: &sqlite::Connection,
    idToProp: &'a mut HashMap<i64, (String, i64)>,
    NameToId: &'a mut HashMap<String, i64>,
) -> Result<(), SimpleError> {
    let q = "select Id, Name, StorageType from SystemIndex_1_PropertyStore_Metadata";
    let s = map_err!(c.prepare(q))?;

    for row in s.into_iter().map(|row| row.unwrap()) {
        let id = row.read::<i64, _>("Id");
        let name = row.read::<&str, _>("Name").to_string();
        let storageType = row.read::<i64, _>("StorageType");

        idToProp.insert(id, (name.clone(), storageType));
        NameToId.insert(name, id);
    }
    Ok(())
}

// This report will provide information about all the files that have been indexed by Windows search,
// including the file name, path, and creation/modification dates.
pub fn sqlite_generate_report(
    f: &Path,
    report_prod: &ReportProducer,
    status_logger: &mut Box<dyn Write>,
) -> Result<(), SimpleError> {
    writeln!(
        status_logger,
        "Processing SQLite db: {}",
        &f.to_string_lossy()
    )
    .map_err(|e| SimpleError::new(format!("{e}")))?;

    let c = map_err!(sqlite::Connection::open_with_flags(
        f,
        sqlite::OpenFlags::new().with_read_only().with_no_mutex()
    ))?;
    let query = "select * from SystemIndex_1_PropertyStore";
    let mut s = map_err!(c.prepare(query))?;

    let recovered_hostname = match sqlite_get_hostname(&c) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("sqlite_get_hostname() failed: {e}. Will use 'Unknown' as a hostname.");
            "Unknown".to_string()
        }
    };

    let (mut file_rep, mut ie_rep, mut act_rep) =
        init_reports(f, report_prod, &recovered_hostname, status_logger, None)?;

    let mut idToProp = HashMap::<i64, (String, i64)>::new();
    let mut propNameToId = HashMap::<String, i64>::new();
    if populate_property_id_maps(&c, &mut idToProp, &mut propNameToId).is_err() {
        panic!("Unable to read property IDs.")
    };

    let mut handler = |workId: u32, record: &mut HashMap<i64, Vec<u8>>| {
        // new WorkId, handle all collected fields
        if !record.is_empty() {
            if is_internet_record(&record, &propNameToId).is_ok() {
                write_record_to_report(record, workId, &idToProp, &mut *ie_rep);
            } else if is_activity_history_record(record, &propNameToId).is_ok() {
                write_record_to_report(record, workId, &idToProp, &mut *act_rep);
            } else {
                write_record_to_report(record, workId, &idToProp, &mut *file_rep);
            }
            record.clear();
        }
    };

    let mut record = HashMap::new();
    let mut workId_current = 0;
    while let Ok(State::Row) = s.next() {
        let workId = map_err!(s.read::<i64, _>("WorkId"))? as u32;
        if workId_current != workId {
            handler(workId_current, &mut record);
            workId_current = workId;
        }
        let columnId = map_err!(s.read::<i64, _>("ColumnId"))?;
        let value = map_err!(s.read::<Vec<u8>, _>("Value"))?;
        record.insert(columnId, value);
    }
    // handle last element
    if !record.is_empty() {
        handler(workId_current, &mut record);
    }
    Ok(())
}

fn write_record_to_report(
    record: &HashMap<i64, Vec<u8>>,
    workId: u32,
    idToProp: &HashMap<i64, (String, i64)>,
    report: &mut dyn Report,
) {
    report.create_new_row();
    report.insert_int_val("WorkId", workId as u64);

    for (col, val) in record.iter().sorted() {
        let property_name = idToProp.get(col);
        if let Some((property_name, storage_type)) = property_name {
            let property_name = property_name.replace(".", "_");
            match storage_type {
                11 => {
                    // inferred to be string type
                    report.insert_str_val(&property_name, String::from_utf8_lossy(val).into_owned())
                }
                12 => {
                    // inferred to be date type when "Date" present in property name
                    if property_name.contains("Date") || property_name.contains("Time") {
                        report.insert_str_val(
                            &property_name,
                            format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
                        )
                    } else {
                        // otherwise inferred to be int type
                        report.insert_int_val(&property_name, u64::from_bytes(val))
                    }
                }
                _ => { /* Storage type not supported. */ }
            }
        }
    }
}

fn is_internet_record(
    record: &HashMap<i64 /*ColumnId*/, Vec<u8> /*Value*/>,
    propNameToId: &HashMap<String, i64>,
) -> Result<(), SimpleError> {
    let targetUriId = propNameToId
        .get("System.Link.TargetUrl")
        .ok_or_else(|| SimpleError::new("Could not find System.Link.TargetUrl ID in map."))?;
    let targetUriVal = record
        .get(targetUriId)
        .ok_or_else(|| SimpleError::new("Could not find System.Link.TargetUrl field in record."))?;
    let uriValStr = String::from_utf8_lossy(targetUriVal).into_owned();
    if !(uriValStr.starts_with("http")) {
        return Err(SimpleError::new(
            "System.Link.TargetUrl does not start with http.",
        ));
    }
    Ok(())
}

fn is_activity_history_record(
    record: &HashMap<i64 /*ColumnId*/, Vec<u8> /*Value*/>,
    propNameToId: &HashMap<String, i64>,
) -> Result<(), SimpleError> {
    let itemTypeId = propNameToId
        .get("System.ItemType")
        .ok_or_else(|| SimpleError::new("Could not find System.ItemType ID in map."))?;
    let itemTypeVal = record
        .get(itemTypeId)
        .ok_or_else(|| SimpleError::new("Could not find System.ItemType field in record."))?;
    let itemTypeStr = String::from_utf8_lossy(itemTypeVal).into_owned();
    if itemTypeStr != "ActivityHistoryItem" {
        return Err(SimpleError::new(
            "System.ItemType is not ActivityHistoryItem.",
        ));
    }
    Ok(())
}

#[test]
fn test_get_property_id_map() {
    let f = "tests/testdata/sqlite/Windows.db";
    let c = map_err!(sqlite::Connection::open_with_flags(
        f,
        sqlite::OpenFlags::new().with_read_only().with_no_mutex()
    ))
    .unwrap();
    let mut idToProp = HashMap::<i64, (String, i64)>::new();
    let mut PropNameToId = HashMap::<String, i64>::new();
    populate_property_id_maps(&c, &mut idToProp, &mut PropNameToId).unwrap();
    assert!(idToProp.len() == 597);
    assert!(PropNameToId.len() == idToProp.len());
}
