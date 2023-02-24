use simple_error::SimpleError;
use std::path::Path;
use std::collections::HashMap;

use crate::report::*;
use crate::utils::*;

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

fn ese_get_first_value_as_string(
    c: &sqlite::Connection,
    table: &str,
    column_id: &str
) -> Result<String, SimpleError> {
    // "557" => r.str_val("System_ComputerName"
    let q = format!("select Value from {table} where ColumnId={column_id} and Value is not NULL and Value <> '' limit 1");
    let mut s = map_err!(c.prepare(q))?;
    if let Ok(State::Row) = s.next() {
        let val = map_err!(s.read::<Vec<u8>, _>("Value"))?;
        return Ok(String::from_utf8_lossy(&val).into_owned());
    }
    Ok("".into())
}

// This report will provide information about all the files that have been indexed by Windows search,
// including the file name, path, and creation/modification dates.
pub fn sqlite_generate_report(f: &Path, report_prod: &ReportProducer) -> Result<(), SimpleError> {
    let c = map_err!(sqlite::Connection::open_with_flags(f,
        sqlite::OpenFlags::new().set_read_only()))?;
    let query = "select * from SystemIndex_1_PropertyStore";
    let mut s = map_err!(c.prepare(query))?;

    //let gather_table_fields = dump_file_gather_sqlite(f)?;

    let recovered_hostname = ese_get_first_value_as_string(
        &c, "SystemIndex_1_PropertyStore", "557" /*System_ComputerName*/)?;

    let (file_rep_path, file_rep) = report_prod.new_report(f, &recovered_hostname, "File_Report")?;
    // declare all headers (using in csv report)
    file_rep.set_field("WorkId");
    file_rep.set_field("System_ComputerName");
    file_rep.set_field("System_ItemPathDisplay");
    file_rep.set_field("System_DateModified");
    file_rep.set_field("System_DateCreated");
    file_rep.set_field("System_DateAccessed");
    file_rep.set_field("System_Size");
    file_rep.set_field("System_FileOwner");
    file_rep.set_field("System_Search_AutoSummary");
    file_rep.set_field("System_Search_GatherTime");
    file_rep.set_field("System_ItemType");

    let (ie_rep_path, ie_rep) = report_prod.new_report(f, &recovered_hostname, "Internet_History_Report")?;
    ie_rep.set_field("WorkId");
    ie_rep.set_field("System_ComputerName");
    ie_rep.set_field("System_ItemName");
    ie_rep.set_field("System_ItemUrl");
    ie_rep.set_field("System_ItemDate");
    ie_rep.set_field("System_DateCreated");
    ie_rep.set_field("System_ItemFolderNameDisplay");
    ie_rep.set_field("System_Search_GatherTime");
    ie_rep.set_field("System_Title");
    ie_rep.set_field("System_Link_DateVisited");

    let (act_rep_path,  act_rep) = report_prod.new_report(f, &recovered_hostname, "Activity_History_Report")?;
    act_rep.set_field("WorkId");
    act_rep.set_field("System_ComputerName");
    act_rep.set_field("System_ItemNameDisplay");
    act_rep.set_field("System_ItemUrl");
    act_rep.set_field("System_ActivityHistory_StartTime");
    act_rep.set_field("System_ActivityHistory_EndTime");
    act_rep.set_field("System_Activity_AppDisplayName");
    act_rep.set_field("System_ActivityHistory_AppId");
    act_rep.set_field("System_Activity_DisplayText");
    act_rep.set_field("VolumeId");
    act_rep.set_field("ObjectId");
    act_rep.set_field("System_Activity_ContentUri");

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
                if ie_rep.is_some_val_in_record() {
                    ie_rep.str_val("System_ComputerName", recovered_hostname.clone());
                }
                if act_rep.is_some_val_in_record() {
                    act_rep.str_val("System_ComputerName", recovered_hostname.clone());
                }
                if file_rep.is_some_val_in_record() {
                    file_rep.str_val("System_ComputerName", recovered_hostname.clone());
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
    r.int_val("WorkId", workId as u64);
    for (col, val) in h {
        match col.as_str() {
            "557" => r.str_val("System_ComputerName", String::from_utf8_lossy(&val).into_owned()),
            "39" => r.str_val("System_ItemPathDisplay", String::from_utf8_lossy(&val).into_owned()),
            "441" => r.str_val("System_DateModified", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "445" => r.str_val("System_DateCreated", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "449" => r.str_val("System_DateAccessed", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "436" => r.int_val("System_Size", u64::from_bytes(&val)),
            "93" => r.str_val("System_FileOwner", String::from_utf8_lossy(&val).into_owned()),
            "303" => r.str_val("System_Search_AutoSummary", String::from_utf8_lossy(&val).into_owned()),
            "26" => r.str_val("System_Search_GatherTime", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "567" => r.str_val("System_ItemType", String::from_utf8_lossy(&val).into_owned()),
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
    let url = h.get_key_value("39");
    if url.is_none() {
        return false;
    }
    if let Some((_, val)) = url {
        let v = String::from_utf8_lossy(&val).into_owned();
        if !(v.starts_with("winrt://") && v.contains("/LS/Desktop/Microsoft Edge/stable/Default/")) {
            return false;
        }
    }
    let name = h.get_key_value("318");
    if name.is_none() {
        return false;
    }
    if let Some((_, val)) = name {
        let v = String::from_utf8_lossy(&val).into_owned();
        if !v.starts_with("http://") && !v.starts_with("https://") {
            return false;
        }
    }
    r.new_record();
    r.int_val("WorkId", workId as u64);
    for (col, val) in h {
        match col.as_str() {
            "557" => r.str_val("System_ComputerName", String::from_utf8_lossy(&val).into_owned()),
            "318" => r.str_val("System_ItemName", String::from_utf8_lossy(&val).into_owned()),
            "39" => r.str_val("System_ItemUrl", String::from_utf8_lossy(&val).into_owned()),
            "308" => r.str_val("System_ItemDate", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "445" => r.str_val("System_DateCreated", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "414" => r.str_val("System_ItemFolderNameDisplay", String::from_utf8_lossy(&val).into_owned()),
            "26" => r.str_val("System_Search_GatherTime", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "424" => r.str_val("System_Title", String::from_utf8_lossy(&val).into_owned()),
            "378" => r.str_val("System_Link_DateVisited", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
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
    r.int_val("WorkId", workId as u64);
    for (col, val) in h {
        match col.as_str() {
            "557" => r.str_val("System_ComputerName", String::from_utf8_lossy(&val).into_owned()),
            "432" => r.str_val("System_ItemNameDisplay", String::from_utf8_lossy(&val).into_owned()),
            "39" => r.str_val("System_ItemUrl", String::from_utf8_lossy(&val).into_owned()),
            "346" => r.str_val("System_ActivityHistory_StartTime", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "341" => r.str_val("System_ActivityHistory_EndTime", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "297" => r.str_val("System_Activity_AppDisplayName", String::from_utf8_lossy(&val).into_owned()),
            "331" => r.str_val("System_ActivityHistory_AppId", String::from_utf8_lossy(&val).into_owned()),
            "315" => r.str_val("System_Activity_DisplayText", String::from_utf8_lossy(&val).into_owned()),
            "311" => {
                let v = String::from_utf8_lossy(&val).into_owned();
                r.str_val("VolumeId", find_guid(&v, "VolumeId="));
                r.str_val("ObjectId", find_guid(&v, "ObjectId="));
                r.str_val("System_Activity_ContentUri", v);
            },
            _ => {}
        }
    }
    true
}