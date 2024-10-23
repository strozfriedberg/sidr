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

fn get_property_id_map<'a>(
    c: &sqlite::Connection,
    m: &'a mut HashMap<i64, (String, i64)>,
) -> Result<(), SimpleError> {
    let q = "select Id, Name, StorageType from SystemIndex_1_PropertyStore_Metadata";
    let s = map_err!(c.prepare(q))?;

    for row in s.into_iter().map(|row| row.unwrap()) {
        // dbg!(row);
        m.insert(
            row.read::<i64, _>("Id"),
            (
                row.read::<&str, _>("Name").to_string(),
                row.read::<i64, _>("StorageType"),
            ),
        );
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

    let mut handler = |workId: u32, h: &mut HashMap<i64, Vec<u8>>| {
        // new WorkId, handle all collected fields
        if !h.is_empty() {
            let ie_history = sqlite_IE_history_record(&mut *ie_rep, workId, h);
            let act_history = sqlite_activity_history_record(&mut *act_rep, workId, h);
            if !ie_history && !act_history {
                // only for File Report
                // Join WorkID within SystemIndex_1_PropertyStore with DocumentID in SystemIndex_Gthr
                // if let Some(gh) = gather_table_fields.get(&workId) {
                //     for (k, v) in gh {
                //         h.insert(k.into(), v.clone());
                //     }
                // }
                sqlite_dump_file_record(&mut *file_rep, workId, h, &c);
            }
            h.clear();
        }
    };

    let mut h = HashMap::new();
    let mut workId_current = 0;
    while let Ok(State::Row) = s.next() {
        let workId = map_err!(s.read::<i64, _>("WorkId"))? as u32;
        if workId_current != workId {
            handler(workId_current, &mut h);
            workId_current = workId;
        }
        let columnId = map_err!(s.read::<i64, _>("ColumnId"))?;
        let value = map_err!(s.read::<Vec<u8>, _>("Value"))?;
        h.insert(columnId, value);
    }
    // handle last element
    if !h.is_empty() {
        handler(workId_current, &mut h);
    }
    Ok(())
}

// File Report
fn sqlite_dump_file_record(
    r: &mut dyn Report,
    workId: u32,
    h: &HashMap<i64 /*ColumnId*/, Vec<u8> /*Value*/>,
    c: &sqlite::Connection,
) {
    r.create_new_row();
    r.insert_int_val("WorkId", workId as u64);
    let mut m = HashMap::<i64, (String, i64)>::new();
    if get_property_id_map(c, &mut m).is_err() {
        panic!("Unable to read property IDs.")
    };

    for (col, val) in h {
        match col {
            39 => r.insert_str_val(
                "System_ItemPathDisplay",
                String::from_utf8_lossy(val).into_owned(),
            ),
            441 => r.insert_str_val(
                "System_DateModified",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            445 => r.insert_str_val(
                "System_DateCreated",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            449 => r.insert_str_val(
                "System_DateAccessed",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            436 => r.insert_int_val("System_Size", u64::from_bytes(val)),
            93 => r.insert_str_val(
                "System_FileOwner",
                String::from_utf8_lossy(val).into_owned(),
            ),
            303 => r.insert_str_val(
                "System_Search_AutoSummary",
                String::from_utf8_lossy(val).into_owned(),
            ),
            26 => r.insert_str_val(
                "System_Search_GatherTime",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            567 => r.insert_str_val("System_ItemType", String::from_utf8_lossy(val).into_owned()),
            557 => r.insert_str_val(
                "System_ComputerName",
                String::from_utf8_lossy(val).into_owned(),
            ),
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
                //     r.insert_str_val(col, format!("{:?}", val));
                // }
            }
        }
    }
}

//IE/Edge History Report
fn sqlite_IE_history_record(
    r: &mut dyn Report,
    workId: u32,
    h: &HashMap<i64 /*ColumnId*/, Vec<u8> /*Value*/>,
) -> bool {
    let url = h.get_key_value(&39);
    if url.is_none() {
        return false;
    }
    if let Some((_, val)) = url {
        let v = String::from_utf8_lossy(val).into_owned();
        if !(v.starts_with("winrt://") && v.contains("/LS/Desktop/Microsoft Edge/stable/Default/"))
        {
            return false;
        }
    }
    let name = h.get_key_value(&318);
    if name.is_none() {
        return false;
    }
    if let Some((_, val)) = name {
        let v = String::from_utf8_lossy(val).into_owned();
        if !v.starts_with("http://") && !v.starts_with("https://") {
            return false;
        }
    }
    r.create_new_row();
    r.insert_int_val("WorkId", workId as u64);
    for (col, val) in h {
        match col {
            318 => r.insert_str_val("System_ItemName", String::from_utf8_lossy(val).into_owned()),
            39 => r.insert_str_val("System_ItemUrl", String::from_utf8_lossy(val).into_owned()),
            308 => r.insert_str_val(
                "System_ItemDate",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            445 => r.insert_str_val(
                "System_DateCreated",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            414 => r.insert_str_val(
                "System_ItemFolderNameDisplay",
                String::from_utf8_lossy(val).into_owned(),
            ),
            26 => r.insert_str_val(
                "System_Search_GatherTime",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            424 => r.insert_str_val("System_Title", String::from_utf8_lossy(val).into_owned()),
            378 => r.insert_str_val(
                "System_Link_DateVisited",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            557 => r.insert_str_val(
                "System_ComputerName",
                String::from_utf8_lossy(val).into_owned(),
            ),
            _ => {}
        }
    }
    true
}

// Activity History Report
fn sqlite_activity_history_record(
    r: &mut dyn Report,
    workId: u32,
    h: &HashMap<i64 /*ColumnId*/, Vec<u8> /*Value*/>,
) -> bool {
    // record only if 567 == "ActivityHistoryItem"
    let item_type = h.get_key_value(&567);
    if item_type.is_none() {
        return false;
    }
    if let Some((_, val)) = item_type {
        let v = String::from_utf8_lossy(val).into_owned();
        if v != "ActivityHistoryItem" {
            return false;
        }
    }
    r.create_new_row();
    r.insert_int_val("WorkId", workId as u64);
    for (col, val) in h {
        match col {
            432 => r.insert_str_val(
                "System_ItemNameDisplay",
                String::from_utf8_lossy(val).into_owned(),
            ),
            39 => r.insert_str_val("System_ItemUrl", String::from_utf8_lossy(val).into_owned()),
            346 => r.insert_str_val(
                "System_ActivityHistory_StartTime",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            341 => r.insert_str_val(
                "System_ActivityHistory_EndTime",
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            297 => r.insert_str_val(
                "System_Activity_AppDisplayName",
                String::from_utf8_lossy(val).into_owned(),
            ),
            331 => r.insert_str_val(
                "System_ActivityHistory_AppId",
                String::from_utf8_lossy(val).into_owned(),
            ),
            315 => r.insert_str_val(
                "System_Activity_DisplayText",
                String::from_utf8_lossy(val).into_owned(),
            ),
            311 => {
                let v = String::from_utf8_lossy(val).into_owned();
                r.insert_str_val("VolumeId", find_guid(&v, "VolumeId="));
                r.insert_str_val("ObjectId", find_guid(&v, "ObjectId="));
                r.insert_str_val("System_Activity_ContentUri", v);
            }
            557 => r.insert_str_val(
                "System_ComputerName",
                String::from_utf8_lossy(val).into_owned(),
            ),
            _ => {}
        }
    }
    true
}

#[test]
fn test_get_property_id_map() {
    let f = "/Users/juliapaluch/dev/sidr/test2/corrupt/Windows.db";
    let c = map_err!(sqlite::Connection::open_with_flags(
        f,
        sqlite::OpenFlags::new().set_read_only()
    ))
    .unwrap();
    let mut m = HashMap::<i64, (String, i64)>::new();
    get_property_id_map(&c, &mut m).unwrap();
    assert!(m.len() > 0);
}
