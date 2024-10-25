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

/*
extern crate sqlite3_sys as ffi;

// get all fields from SystemIndex_Gthr table
fn dump_file_gather_sqlite(f: &Path)
    -> Result<HashMap<u32/*DocumentID UNSIGNEDLONG_INTEGER*/, HashMap<String, Vec<u8>>>/*rest fields*/, SimpleError>
{
    let mut res : HashMap<u32, HashMap<String, Vec<u8>>> = HashMap::new();

    let gfn = format!("{}/{}-gather.{}", f.parent().unwrap().to_string_lossy(), f.file_stem().unwrap().to_string_lossy(), f.extension().unwrap().to_string_lossy());
    let c = map_err!(sqlite::Connection::open_with_flags(gfn,
        sqlite::OpenFlags::new().set_read_only()))?;

    {
        // setup collation func
        use libc::{c_int, c_void};
        extern "C" fn xCompare (_: *mut c_void, _: c_int, _: *const c_void, _: c_int, _: *const c_void) -> c_int {
            unimplemented!("xCompare")
        }
        use std::ffi::CString;
        let z_name = CString::new("UNICODE_en-US_LINGUISTIC_IGNORECASE").expect("CString::new failed");
        let handle = c.as_raw();
        let _ = unsafe {
            ffi::sqlite3_create_collation(handle, z_name.as_ptr(), ffi::SQLITE_UTF16LE, std::ptr::null_mut(), Some(xCompare))
        };
    }

    let query = "select * from SystemIndex_Gthr";
    let mut s = map_err!(c.prepare(query))?;
    while let Ok(State::Row) = s.next() {
        let mut h = HashMap::new();
        let mut docId = 0;
        for (col_name, col_index) in &*s.column_mapping() {
            if col_name == "DocumentID" {
                docId = map_err!(s.read::<i64, _>(*col_index))? as u32;
            } else {
                let v = map_err!(s.read::<Vec<u8>, _>(*col_index))?;
                if !v.is_empty() {
                    h.insert(col_name.clone(), v);
                }
            }
        }
        res.insert(docId, h);
    }
    Ok(res)
}
*/

fn sqlite_get_hostname(c: &sqlite::Connection) -> Result<String, SimpleError> {
    // ASDF-5849
    // 557 - System_ComputerName
    // 567 - System_ItemType
    let q = "select WorkId as wId, Value from SystemIndex_1_PropertyStore where ColumnId=557 and Value is not NULL and Value <> '' and \
        (select Value from SystemIndex_1_PropertyStore where WorkId=wId and ColumnId=567) <> '.url' order by WorkId desc limit 1;".to_string();
    let mut s = map_err!(c.prepare(q))?;
    if let Ok(State::Row) = s.next() {
        let val = map_err!(s.read::<Vec<u8>, _>("Value"))?;
        return Ok(String::from_utf8_lossy(&val).into_owned());
    }
    Err(SimpleError::new(
        "Empty field System_ComputerName".to_string(),
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
        sqlite::OpenFlags::new().set_read_only()
    ))?;
    let query = "select * from SystemIndex_1_PropertyStore";
    let mut s = map_err!(c.prepare(query))?;

    //let gather_table_fields = dump_file_gather_sqlite(f)?;

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
            if is_internet_record(&record, &propNameToId).is_some() {
                write_record_to_report(record, workId, &idToProp, &mut *ie_rep);
            }
            else if is_activity_history_record(record, &propNameToId).is_some() {
                write_record_to_report(record, workId, &idToProp, &mut *act_rep);
            }
            else {
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

    for (col, val) in record {
        let property_name = idToProp.get(col);
        if let Some((property_name, storage_type)) = property_name {
            match storage_type {
                11 => {
                    report.insert_str_val(property_name, String::from_utf8_lossy(val).into_owned())
                }
                12 => {
                    if property_name.contains("Date") {
                        report.insert_str_val(
                            property_name,
                            format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
                        )
                    } else {
                        report.insert_int_val(property_name, u64::from_bytes(val))
                    }
                }
                _ => { /* Storage type not supported. */},
            }
        }
    }
}

fn is_internet_record(
    record: &HashMap<i64 /*ColumnId*/, Vec<u8> /*Value*/>,
    propNameToId: &HashMap<String, i64>,
) -> Option<()> {
    let itemFolderNameId = propNameToId.get("System.ItemFolderNameDisplay")?;
    let folderNameVal = record.get(itemFolderNameId)?;
    let folderNameStr = String::from_utf8_lossy(folderNameVal).into_owned();
    if !["RecentlyClosed", "History", "QuickLinks"].contains(&&*folderNameStr) {
        return None;
    }

    let targetUriId = propNameToId.get("System.Link.TargetUrl")?;
    let targetUriVal = record.get(targetUriId)?;
    let uriValStr = String::from_utf8_lossy(targetUriVal).into_owned();
    if !(uriValStr.starts_with("http")) {
        return None;
    }
    Some(())
}

fn is_activity_history_record(
    record: &HashMap<i64 /*ColumnId*/, Vec<u8> /*Value*/>,
    propNameToId: &HashMap<String, i64>,
) -> Option<()> {
    let itemTypeId = propNameToId.get("System.ItemType")?;
    let itemTypeVal = record.get(itemTypeId)?;
    let itemTypeStr = String::from_utf8_lossy(itemTypeVal).into_owned();
    if itemTypeStr != "ActivityHistoryItem" {
        return None;
    }
    Some(())
}

#[test]
fn test_get_property_id_map() {
    let f = "/Users/juliapaluch/dev/sidr/test2/corrupt/Windows.db";
    let c = map_err!(sqlite::Connection::open_with_flags(
        f,
        sqlite::OpenFlags::new().set_read_only()
    ))
    .unwrap();
    let mut idToProp = HashMap::<i64, (String, i64)>::new();
    let mut PropNameToId = HashMap::<String, i64>::new();
    populate_property_id_maps(&c, &mut idToProp, &mut PropNameToId).unwrap();
    assert!(idToProp.len() > 0);
}
