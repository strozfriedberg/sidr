use simple_error::SimpleError;
use std::path::Path;
use std::collections::HashMap;

use crate::report::*;
use crate::utils::*;
use crate::fields::*;

use ese_parser_lib::ese_parser::FromBytes;
use sqlite::State;

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

// This report will provide information about all the files that have been indexed by Windows search,
// including the file name, path, and creation/modification dates.
pub fn sqlite_generate_report(f: &Path, format: &ReportFormat) -> Result<(), SimpleError> {
    let c = map_err!(sqlite::Connection::open_with_flags(f,
        sqlite::OpenFlags::new().set_read_only()))?;
    let query = "select * from SystemIndex_1_PropertyStore";
    let mut s = map_err!(c.prepare(query))?;

    //let gather_table_fields = dump_file_gather_sqlite(f)?;

    let (file_rep_path, file_rep) = make_report_format(f, "file-report", format)?;
    // declare all headers (using in csv report)
    file_rep.set_field(WORKID);
    file_rep.set_field(FULL_PATH);
    file_rep.set_field(DATE_MODIFIED);
    file_rep.set_field(DATE_CREATED);
    file_rep.set_field(DATE_ACCESSED);
    file_rep.set_field(SIZE);
    file_rep.set_field(USER);
    file_rep.set_field(CONTENT);
    file_rep.set_field(FILE_ATTRIBUTES);

    let (ie_rep_path, ie_rep) = make_report_format(f, "ie-report", format)?;
    ie_rep.set_field(WORKID);
    ie_rep.set_field(URL);
    ie_rep.set_field(FULL_PATH_URL);
    ie_rep.set_field(SYSTEM_TIME_OF_THE_VISIT);
    ie_rep.set_field(DATE_CREATED);
    ie_rep.set_field(TYPE_OF_ACTIVITY);

    let (act_rep_path,  act_rep) = make_report_format(f, "act-report", format)?;
    act_rep.set_field(WORKID);
    act_rep.set_field(ACTIVITYHISTORY_IDENTIFIER);
    act_rep.set_field(ACTIVITYHISTORY_FILENAME);
    act_rep.set_field(ACTIVITYHISTORY_FULLPATH);
    act_rep.set_field(ACTIVITY_START_TIMESTAMP);
    act_rep.set_field(ACTIVITY_END_TIMESTAMP);
    act_rep.set_field(LOCAL_START_TIME);
    act_rep.set_field(LOCAL_END_TIME);
    act_rep.set_field(APPLICATION_NAME);
    act_rep.set_field(APPLICATION_GUID);
    act_rep.set_field(ASSOCIATED_FILE);
    act_rep.set_field(VOLUME_ID);
    act_rep.set_field(OBJECT_ID);
    act_rep.set_field(FULLPATH_ASSOCIATED_FILE);

    eprintln!("{}\n{}\n{}\n", file_rep_path.to_string_lossy(), ie_rep_path.to_string_lossy(), act_rep_path.to_string_lossy());

    let mut h = HashMap::new();
    let mut workId_current = 0;
    while let Ok(State::Row) = s.next() {
        let workId = map_err!(s.read::<i64, _>("WorkId"))? as u32;
        if workId_current != workId {
            // new WorkId, handle all collected fields
            if !h.is_empty() {
                if !sqlite_activity_history_record(&act_rep, workId_current, &h) && !sqlite_IE_history_record(&ie_rep, workId_current, &h) {
                    // only for File Report
                    // Join WorkID within SystemIndex_1_PropertyStore with DocumentID in SystemIndex_Gthr
                    // if let Some(gh) = gather_table_fields.get(&workId_current) {
                    //     for (k, v) in gh {
                    //         h.insert(k.into(), v.clone());
                    //     }
                    // }
                    sqlite_dump_file_record(&file_rep, workId_current, &h);
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

// File Report
fn sqlite_dump_file_record(r: &Box<dyn Report>, workId: u32, h: &HashMap<String/*ColumnId*/, Vec<u8>/*Value*/>) {
    r.new_record();
    r.int_val(WORKID, workId as u64);
    for (col, val) in h {
        match col.as_str() {
            "39" => r.str_val(FULL_PATH, String::from_utf8_lossy(&val).into_owned()),
            "441" => r.str_val(DATE_MODIFIED, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "445" => r.str_val(DATE_CREATED, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "449" => r.str_val(DATE_ACCESSED, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "436" => r.int_val(SIZE, u64::from_bytes(&val)),
            "93" => r.str_val(USER, String::from_utf8_lossy(&val).into_owned()),
            "303" => r.str_val(CONTENT, format!("{:02X?}", val)), // TODO: decompress
            "438" => r.str_val(FILE_ATTRIBUTES, file_attributes_to_string(val)),
            // "ScopeID" => println!("{}", col, i32::from_bytes(val)),
            // "DocumentID" => println!("{}", col, i32::from_bytes(val)),
            // "SDID" => println!("{}", col, i32::from_bytes(val)),
            // "LastModified" => println!("{}", col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            // "TransactionFlags" => println!("{}", col, i32::from_bytes(val)),
            // "TransactionExtendedFlags" => println!("{}", col, i32::from_bytes(val)),
            // "CrawlNumberCrawled" => println!("{}", col, i32::from_bytes(val)),
            // "StartAddressIdentifier" => println!("{}", col, u16::from_bytes(val)),
            // "Priority" => println!("{}", col, u8::from_bytes(val)),
            // "FileName" => println!("{}", col, from_utf16(val)),
            // "DeletedCount" => println!("{}", col, i32::from_bytes(val)),
            // "RunTime" => println!("{}", col, i32::from_bytes(val)),
            // "FailureUpdateAttempts" => println!("{}", col, u8::from_bytes(val)),
            // "ClientID" => println!("{}", col, u32::from_bytes(val)),
            // "LastRequestedRunTime" => println!("{}", col, u32::from_bytes(val)),
            // "CalculatedPropertyFlags" => println!("{}", col, u32::from_bytes(val)),
            _ => {
                // /*
                // field: UserData
                // field: AppOwnerId
                // field: RequiredSIDs
                // field: StorageProviderId
                // */
                // if col.chars().nth(0).unwrap().is_alphabetic() {
                //     r.str_val(col, format!("{:?}", val));
                // }
            }
        }
    }
}

//IE/Edge History Report
fn sqlite_IE_history_record(r: &Box<dyn Report>, workId: u32, h: &HashMap<String/*ColumnId*/, Vec<u8>/*Value*/>) -> bool {
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
    r.new_record();
    r.int_val(WORKID, workId as u64);
    for (col, val) in h {
        match col.as_str() {
            "318" => r.str_val(URL, String::from_utf8_lossy(&val).into_owned()),
            "39" => r.str_val(FULL_PATH_URL, String::from_utf8_lossy(&val).into_owned()),
            "308" => r.str_val(SYSTEM_TIME_OF_THE_VISIT, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "445" => r.str_val(DATE_CREATED, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "414" => r.str_val(TYPE_OF_ACTIVITY, String::from_utf8_lossy(&val).into_owned()),
            _ => {}
        }
    }
    true
}

// Activity History Report
fn sqlite_activity_history_record(r: &Box<dyn Report>, workId: u32, h: &HashMap<String/*ColumnId*/, Vec<u8>/*Value*/>) -> bool {
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
    r.new_record();
    r.int_val(WORKID, workId as u64);
    for (col, val) in h {
        match col.as_str() {
            "567" => r.str_val(ACTIVITYHISTORY_IDENTIFIER, String::from_utf8_lossy(&val).into_owned()),
            "432" => r.str_val(ACTIVITYHISTORY_FILENAME, String::from_utf8_lossy(&val).into_owned()),
            "39" => r.str_val(ACTIVITYHISTORY_FULLPATH, String::from_utf8_lossy(&val).into_owned()),
            "346" => r.str_val(ACTIVITY_START_TIMESTAMP, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "341" => r.str_val(ACTIVITY_END_TIMESTAMP, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "353" => r.str_val(LOCAL_START_TIME, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "355" => r.str_val(LOCAL_END_TIME, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "297" => r.str_val(APPLICATION_NAME, String::from_utf8_lossy(&val).into_owned()),
            "331" => r.str_val(APPLICATION_GUID, String::from_utf8_lossy(&val).into_owned()),
            "315" => r.str_val(ASSOCIATED_FILE, String::from_utf8_lossy(&val).into_owned()),
            "311" => {
                let v = String::from_utf8_lossy(&val).into_owned();
                r.str_val(VOLUME_ID, find_guid(&v, "VolumeId="));
                r.str_val(OBJECT_ID, find_guid(&v, "ObjectId="));
                r.str_val(FULLPATH_ASSOCIATED_FILE, v);
            },
            _ => {}
        }
    }
    true
}