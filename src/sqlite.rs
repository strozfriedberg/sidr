use simple_error::SimpleError;
use std::path::Path;
use std::collections::HashMap;

use crate::utils::*;

use ese_parser_lib::ese_parser::FromBytes;
use sqlite::State;

macro_rules! map_err(($result:expr) => ($result.map_err(|e| SimpleError::new(format!("{}", e)))));

fn sqlite_dump_file_record(workId: u32, h: &HashMap<String/*ColumnId*/, Vec<u8>/*Value*/>) {
    println!("File Report for WorkId/DocumentId {}", workId);
    for (col, val) in h {
        match col.as_str() {
            "39" => println!("Full Path: {}", String::from_utf8_lossy(&val).into_owned()),
            "441" => println!("Date Modified: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "445" => println!("Date Created: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "449" => println!("Date Accessed: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "436" => println!("Size: {}", u64::from_bytes(&val)),
            "93" => println!("User: {}", String::from_utf8_lossy(&val).into_owned()),
            "303" => println!("Partial Content of File: {:02X?}", val), // TODO: decompress
            "438" => println!("File Attributes: {:?}", val), // TODO: pretty print? E.g. FILE_ATTRIBUTE_READONLY, etc.
            // "ScopeID" => println!("{}: {}", col, i32::from_bytes(val)),
            // "DocumentID" => println!("{}: {}", col, i32::from_bytes(val)),
            // "SDID" => println!("{}: {}", col, i32::from_bytes(val)),
            // "LastModified" => println!("{}: {}", col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            // "TransactionFlags" => println!("{}: {}", col, i32::from_bytes(val)),
            // "TransactionExtendedFlags" => println!("{}: {}", col, i32::from_bytes(val)),
            // "CrawlNumberCrawled" => println!("{}: {}", col, i32::from_bytes(val)),
            // "StartAddressIdentifier" => println!("{}: {}", col, u16::from_bytes(val)),
            // "Priority" => println!("{}: {}", col, u8::from_bytes(val)),
            // "FileName" => println!("{}: {}", col, from_utf16(val)),
            // "DeletedCount" => println!("{}: {}", col, i32::from_bytes(val)),
            // "RunTime" => println!("{}: {}", col, i32::from_bytes(val)),
            // "FailureUpdateAttempts" => println!("{}: {}", col, u8::from_bytes(val)),
            // "ClientID" => println!("{}: {}", col, u32::from_bytes(val)),
            // "LastRequestedRunTime" => println!("{}: {}", col, u32::from_bytes(val)),
            // "CalculatedPropertyFlags" => println!("{}: {}", col, u32::from_bytes(val)),
            _ => {
                /*
                field: UserData
                field: AppOwnerId
                field: RequiredSIDs
                field: StorageProviderId
                */
                if col.chars().nth(0).unwrap().is_alphabetic() {
                    println!("{}: {:?}", col, val);
                }
            }
        }
    }
    println!("");
}

fn sqlite_IE_history_record(workId: u32, h: &HashMap<String/*ColumnId*/, Vec<u8>/*Value*/>) -> bool {
    // record only if 39 starts with iehistory://
    let item_type = h.get_key_value("39");
    if item_type.is_none() {
        return false;
    }
    if let Some((_, val)) = item_type {
        let v = String::from_utf8_lossy(&val).into_owned();
        if !v.starts_with("winrt://") {
            return false;
        }
    }
    println!("IE/Edge History Report for WorkId {}", workId);
    for (col, val) in h {
        match col.as_str() {
            "318" => println!("URL: {}", String::from_utf8_lossy(&val).into_owned()),
            "39" => println!("Full Path of the URL: {}", String::from_utf8_lossy(&val).into_owned()),
            "308" => println!("System Time of the visit: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "445" => println!("Date Created (For Win 11): {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "414" => println!("Type of activity (for Win 11): {:?}", String::from_utf8_lossy(&val).into_owned()),
            _ => {}
        }
    }
    println!("");
    true
}

fn sqlite_activity_history_record(workId: u32, h: &HashMap<String/*ColumnId*/, Vec<u8>/*Value*/>) -> bool {
    // record only if 567 == "ActivityHistoryItem"
    let item_type = h.get_key_value("567");
    if item_type.is_none() {
        return false;
    }
    if let Some((_, val)) = item_type {
        let v = String::from_utf8_lossy(&val).into_owned();
        if v != "ActivityHistoryItem" {
            return false;
        }
    }
    println!("Activity History Report for WorkId {}", workId);
    for (col, val) in h {
        match col.as_str() {
            "567" => println!("ActivityHistory Identifier: {}", String::from_utf8_lossy(&val).into_owned()),
            "432" => println!("ActivityHistory FileName: {}", String::from_utf8_lossy(&val).into_owned()),
            "39" => println!("ActivityHistory FullPath: {}", String::from_utf8_lossy(&val).into_owned()),
            "346" => println!("Activity Start Timestamp: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "341" => println!("Activity End Timestamp: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "353" => println!("Local Start Time: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "355" => println!("Local End Time: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "297" => println!("Application Name: {}", String::from_utf8_lossy(&val).into_owned()),
            "331" => println!("Application GUID: {}", String::from_utf8_lossy(&val).into_owned()),
            "315" => println!("Associated File: {}", String::from_utf8_lossy(&val).into_owned()),
            "311" => {
                let v = String::from_utf8_lossy(&val).into_owned();
                println!("FullPath of the Assocaited File: {}", v);
                println!("Volumd ID: {}", find_guid(&v, "VolumeId="));
                println!("Object ID: {}", find_guid(&v, "ObjectId="));
            },
            _ => {}
        }
    }
    println!("");
    true
}

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

// This report will provide information about all the files that have been indexed by Windows search,
// including the file name, path, and creation/modification dates.
pub fn sqlite_generate_report(f: &Path) -> Result<(), SimpleError> {
    let c = map_err!(sqlite::Connection::open_with_flags(f,
        sqlite::OpenFlags::new().set_read_only()))?;
    let query = "select * from SystemIndex_1_PropertyStore";
    let mut s = map_err!(c.prepare(query))?;

    //let gather_table_fields = dump_file_gather_sqlite(f)?;

    let mut h = HashMap::new();
    let mut workId_current = 0;
    while let Ok(State::Row) = s.next() {
        let workId = map_err!(s.read::<i64, _>("WorkId"))? as u32;
        if workId_current != workId {
            // new WorkId, handle all collected fields
            if !h.is_empty() {
                if !sqlite_activity_history_record(workId_current, &h) && !sqlite_IE_history_record(workId_current, &h) {
                    // only for File Report
                    // Join WorkID within SystemIndex_1_PropertyStore with DocumentID in SystemIndex_Gthr
                    // if let Some(gh) = gather_table_fields.get(&workId_current) {
                    //     for (k, v) in gh {
                    //         h.insert(k.into(), v.clone());
                    //     }
                    // }
                    sqlite_dump_file_record(workId_current, &h);
                }
                h.clear();
            }
            workId_current = workId;
        }
        let columnId = map_err!(s.read::<i64, _>("ColumnId"))?;
        let value = map_err!(s.read::<Vec<u8>, _>("Value"))?;
        h.insert(columnId.to_string(), value);
    }
    Ok(())
}
