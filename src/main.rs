#![allow(
    unused,
    non_upper_case_globals,
    non_snake_case,
    non_camel_case_types,
)]

use std::path::Path;
use std::ffi::OsStr;
use std::collections::HashMap;

use ese_parser_lib::ese_trait::*;
use simple_error::SimpleError;
use std::{cell::RefCell, borrow::Borrow};

use sqlite::{Statement, State, CursorWithOwnership, Cursor};

macro_rules! map_err(($result:expr) => ($result.map_err(|e| SimpleError::new(format!("{}", e)))));

// return hashmap to map
// Windows.edb (ESE db) fields (like "4447-System_ItemPathDisplay") to
// Windows.db (sqlite db) fields (like ColumnID=39)
//fn get_mapping()

use chrono::prelude::*;

/// Converts a u64 filetime to a DateTime<Utc>
pub fn get_date_time_from_filetime(filetime: u64) -> DateTime<Utc> {
    const UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH: i128 = 11644473600;
    const UNIX_EPOCH_NANOS: i128 = UNIX_EPOCH_SECONDS_SINCE_WINDOWS_EPOCH * 1_000_000_000;
    let filetime_nanos: i128 = filetime as i128 * 100;

    // Add nanoseconds to timestamp via Duration
    DateTime::<Utc>::from_utc(
        chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().and_hms_nano_opt(0, 0, 0, 0).unwrap()
            + chrono::Duration::nanoseconds((filetime_nanos - UNIX_EPOCH_NANOS) as i64),
        Utc,
    )
}

/// Converts a DateTime<Utc> to ISO-8601/RFC-3339 format `%Y-%m-%dT%H:%M:%S%.7f` (manually, since Rust doesn't support `%.7f`)
pub fn format_date_time(date_time: DateTime<Utc>) -> String {
    let fractional_seconds = date_time.format("%9f").to_string();
    const EXPECTED_FRACTIONAL_SECONDS_LEN: usize = 9;
    if EXPECTED_FRACTIONAL_SECONDS_LEN == fractional_seconds.len() {
        let byte_slice = fractional_seconds.as_bytes(); // we know that the string is only ASCII, so this is safe
                                                        // Make sure that our last two digits are 0, as we expect
                                                        // Note that we aren't just using chrono::SecondsFormat::AutoSi because we want 7 digits to correspond to the original filetime's 100ns precision
        if byte_slice[EXPECTED_FRACTIONAL_SECONDS_LEN - 1] == b'0'
            && byte_slice[EXPECTED_FRACTIONAL_SECONDS_LEN - 2] == b'0'
        {
            return format!(
                "{}.{}Z",
                date_time.format("%Y-%m-%dT%H:%M:%S"),
                &fractional_seconds[..7]
            );
        }
    }
    // We should nenver hit this when coming from a FILETIME; we don't have that much precision
    date_time.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}


fn sqlite_dump_file_record(workId: i64, h: &HashMap<i64/*ColumnId*/, Vec<u8>/*Value*/>) {
    println!("File Report for WorkId {}", workId);
    for (colId, val) in h {
        match colId {
            39 => println!("Full Path: {}", String::from_utf8_lossy(&val).into_owned()),
            441 => println!("Date Modified: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            445 => println!("Date Created: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            449 => println!("Date Accessed: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            436 => println!("Size: {}", u64::from_bytes(&val)),
            93 => println!("User: {}", String::from_utf8_lossy(&val).into_owned()),
            303 => println!("Partial Content of File: {:02X?}", val), // TODO: decompress
            438 => println!("File Attributes: {:?}", val), // TODO: pretty print? E.g. FILE_ATTRIBUTE_READONLY, etc.
            _ => {} // println!("Unknown field {}", colId)
            // we got all fields here
            // TODO other tables
        }
    }
    println!("");
}

fn sqlite_IE_history_record(workId: i64, h: &HashMap<i64/*ColumnId*/, Vec<u8>/*Value*/>) {
    println!("IE/Edge History Report for WorkId {}", workId);
    for (colId, val) in h {
        match colId {
            318 => println!("URL: {}", String::from_utf8_lossy(&val).into_owned()),
            39 => println!("Full Path of the URL: {}", String::from_utf8_lossy(&val).into_owned()),
            308 => println!("System Time of the visit: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            445 => println!("Date Created (For Win 11): {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            414 => println!("Type of activity (for Win 11): {:?}", String::from_utf8_lossy(&val).into_owned()),
            _ => {} // println!("Unknown field {}", colId)
        }
    }
    println!("");
}

fn sqlite_activity_history_record(workId: i64, h: &HashMap<i64/*ColumnId*/, Vec<u8>/*Value*/>) {
    println!("Activity History Report for WorkId {}", workId);
    for (colId, val) in h {
        match colId {
            567 => println!("ActivityHistory Identifier: {}", String::from_utf8_lossy(&val).into_owned()),
            432 => println!("ActivityHistory FileName: {}", String::from_utf8_lossy(&val).into_owned()),
            39 => println!("ActivityHistory FullPath: {}", String::from_utf8_lossy(&val).into_owned()),
            346 => println!("Activity Start Timestamp: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            341 => println!("Activity End Timestamp: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            353 => println!("Local Start Time: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            355 => println!("Local End Time: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            297 => println!("Application Name: {}", String::from_utf8_lossy(&val).into_owned()),
            331 => println!("Application GUID: {}", String::from_utf8_lossy(&val).into_owned()),
            315 => println!("Associated File: {}", String::from_utf8_lossy(&val).into_owned()),
            311 => println!("FullPath of the Assocaited File (+Volumd ID, +Object ID): {}", String::from_utf8_lossy(&val).into_owned()),
            _ => {} // println!("Unknown field {}", colId)
        }
    }
    println!("");
}

fn from_utf16(val: &Vec<u8>) -> String {
    let s: Vec<u16> = val
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();
    String::from_utf16_lossy(s.as_slice())
}

fn ese_dump_file_record(workId: u32, h: &HashMap<String, Vec<u8>>) {
    println!("File Report for WorkId {}", workId);
    for (col, val) in h {
        match col.as_str() {
            "4447-System_ItemPathDisplay" => println!("Full Path: {}", from_utf16(val)),
            "15F-System_DateModified" => println!("Date Modified: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "16F-System_DateCreated" => println!("Date Created: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "17F-System_DateAccessed" => println!("Date Accessed: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "13F-System_Size" => println!("Size: {}", u64::from_bytes(&val)),
            "4396-System_FileOwner" => println!("User: {}", from_utf16(&val)),
            "4625-System_Search_AutoSummary" => println!("Partial Content of File: {:02X?}", val),
            "14F-System_FileAttributes" => println!("File Attributes: {:?}", val), // TODO: pretty print? E.g. FILE_ATTRIBUTE_READONLY, etc.
            "FileName" => println!("{}: {}", col, from_utf16(val)),
            _ => {
                /*
                field: ScopeID
                field: DocumentID
                field: SDID
                field: LastModified
                field: TransactionFlags
                field: TransactionExtendedFlags
                field: CrawlNumberCrawled
                field: StartAddressIdentifier
                field: Priority
                field: FileName
                field: UserData
                field: AppOwnerId
                field: RequiredSIDs
                field: DeletedCount
                field: RunTime
                field: FailureUpdateAttempts
                field: ClientID
                field: LastRequestedRunTime
                field: StorageProviderId
                field: CalculatedPropertyFlags
                */
                if (col.chars().nth(0).unwrap().is_alphabetic()) {
                    println!("{}: {:?}", col, val);
                }
            }
        }
    }
    println!("");
}

fn ese_IE_history_record(workId: u32, h: &HashMap<String, Vec<u8>>) {
    println!("IE/Edge History Report for WorkId {}", workId);
    for (col, val) in h {
        match col.as_str() {
            "4442-System_ItemName" => println!("URL: {}", from_utf16(val)),
            "4447-System_ItemPathDisplay" => println!("URL(ItemPathDisplay): {}", from_utf16(val)),
            "15F-System_DateModified" => println!("Modified time: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "33-System_ItemUrl" => println!("Full Path of the URL: {}", from_utf16(val)),
            "4468-System_Link_TargetUrl" => println!("Full Path of the URL (TargetUrl): {}", from_utf16(val)),
            "4438-System_ItemDate" => println!("System Time of the visit: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "4470-System_Link_TargetUrlPath" => println!("TargetUrl: {}", from_utf16(val)),
            _ => {}
        }
    }
    println!("");
}

fn ese_activity_history_record(workId: u32, h: &HashMap<String, Vec<u8>>) {
    println!("Activity History Report for WorkId {}", workId);
    for (col, val) in h {
        match col.as_str() {
            "4450-System_ItemType" => println!("ActivityHistory Identifier: {}", from_utf16(val)),
            "4443-System_ItemNameDisplay" => println!("ActivityHistory FileName: {}", from_utf16(val)),
            "33-System_ItemUrl" => println!("ActivityHistory FullPath: {}", from_utf16(val)), // TODO: get UserSID from here
            "4139-System_ActivityHistory_StartTime" => println!("Activity Start Timestamp: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "4130-System_ActivityHistory_EndTime" => println!("Activity End Timestamp: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "4137-System_ActivityHistory_LocalStartTime" => println!("Local Start Time: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "4136-System_ActivityHistory_LocalEndTime" => println!("Local End Time: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "4105-System_Activity_AppDisplayName" => println!("Application Name: {}", from_utf16(val)),
            "4123-System_ActivityHistory_AppId" => println!("Application GUID: {}", from_utf16(val)),
            "4115-System_Activity_DisplayText" => println!("Associated File: {}", from_utf16(val)),
            "4112-System_Activity_ContentUri" => println!("FullPath of the Assocaited File (+Volumd ID, +Object ID): {}", from_utf16(val)),
            _ => {}
        }
    }
    println!("");
}

//
// TODO:
// Join WorkID within SystemIndex_1_PropertyStore table (Windows.db) with DocumentID in
// SystemIndex_Gthr table (Windows-gather.db)
//

// This report will provide information about all the files that have been indexed by Windows search,
// including the file name, path, and creation/modification dates.
fn dump_file_report_sqlite(f: &Path) -> Result<(), SimpleError> {
    let c = map_err!(sqlite::Connection::open_with_flags(f,
        sqlite::OpenFlags::new().set_read_only()))?;
    let query = "select * from SystemIndex_1_PropertyStore";
    let mut s = map_err!(c.prepare(query))?;
    let mut h = HashMap::new();
    let mut workId_current = 0;
    while let Ok(State::Row) = s.next() {
        let workId = map_err!(s.read::<i64, _>("WorkId"))?;
        if workId_current != workId {
            // new WorkId, handle all collected fields
            if !h.is_empty() {
                sqlite_dump_file_record(workId_current, &h);
                sqlite_IE_history_record(workId_current, &h);
                sqlite_activity_history_record(workId_current, &h);
                h = HashMap::new();
            }
            workId_current = workId;
        }
        let columnId = map_err!(s.read::<i64, _>("ColumnId"))?;
        let value = map_err!(s.read::<Vec<u8>, _>("Value"))?;
        h.insert(columnId, value);
    }
    Ok(())
}

use ese_parser_lib::ese_parser::EseParser;
const CACHE_SIZE_ENTRIES: usize = 10;

fn prepare_selected_cols(cols: Vec<ColumnInfo>, sel_cols: &Vec<&str>) -> Vec<ColumnInfo> {
    let mut only_cols : Vec<ColumnInfo> = Vec::new();
    for c in cols {
        for sc in sel_cols {
            if *sc == c.name {
                only_cols.push(c);
                break;
            }
        }
    }
    if (sel_cols.len() != only_cols.len()) {
        for i in sel_cols {
            let mut found = false;
            for j in &only_cols {
                if *i == j.name {
                    found = true;
                    break;
                }
            }
            if (!found) {
                println!("Requested column {} not found in table columns", i);
            }
        }
    }
    only_cols
}

fn get_column<T: FromBytes>(
    jdb: &dyn EseDb,
    table: u64,
    column: &ColumnInfo,
) -> Result<Option<T>, SimpleError> {
    match jdb.get_column(table, column.id)? {
        Some(v) => Ok(Some(T::from_bytes(&v))),
        None => Ok(None),
    }
}

// get all fields from SystemIndex_Gthr table
/*
field: ScopeID
field: DocumentID
field: SDID
field: LastModified
field: TransactionFlags
field: TransactionExtendedFlags
field: CrawlNumberCrawled
field: StartAddressIdentifier
field: Priority
field: FileName
field: UserData
field: AppOwnerId
field: RequiredSIDs
field: DeletedCount
field: RunTime
field: FailureUpdateAttempts
field: ClientID
field: LastRequestedRunTime
field: StorageProviderId
field: CalculatedPropertyFlags
 */
fn dump_file_gather_ese(f: &Path)
    -> Result<HashMap<u32/*DocumentID UNSIGNEDLONG_INTEGER*/, HashMap<String, Vec<u8>>>/*rest fields*/, SimpleError>
{
    let mut res : HashMap<u32, HashMap<String, Vec<u8>>> = HashMap::new();
    let jdb = Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, f).unwrap());
    let t = "SystemIndex_Gthr";
    let table_id = jdb.open_table(t)?;
    let cols = jdb.get_columns(t)?;
    if !jdb.move_row(table_id, ESE_MoveFirst)? {
        // empty table
        //return Err(SimpleError::new(format!("Empty table {t}")));
        return Ok(res);
    }
    // find DocumentID column
    let docID_indx = cols.iter().position(|r| r.name == "DocumentID")
        .ok_or(SimpleError::new("Could't locate DocumentID field"))?;
    loop {
        let mut h = HashMap::new();
        if let Some(docId) = get_column::<u32>(&*jdb, table_id, &cols[docID_indx])? {
            for i in 0..cols.len() {
                if i == docID_indx {
                    continue;
                }
                match jdb.get_column(table_id, cols[i].id) {
                    Ok(val) => match val {
                        None => {} // println!("Empty field: {}", cols[i].name),
                        Some(v) => {
                            h.insert(cols[i].name.clone(), v);
                        }
                    },
                    Err(e) => println!("Error while getting column {} from {}", cols[i].name, t)
                }
            }
            res.insert(docId, h);
        }
        if !jdb.move_row(table_id, ESE_MoveNext)? {
            break;
        }
    }
    Ok(res)
}

fn dump_file_report_ese(f: &Path) -> Result<(), SimpleError> {
    let jdb = Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, f).unwrap());
    let t = "SystemIndex_PropertyStore";
    let table_id = jdb.open_table(t)?;
    let cols = jdb.get_columns(t)?;
    if !jdb.move_row(table_id, ESE_MoveFirst)? {
        // empty table
        return Err(SimpleError::new(format!("Empty table {t}")));
    }
    let gather_table_fields = dump_file_gather_ese(f)?;

    // prepare to query only selected columns
    let sel_cols = prepare_selected_cols(cols,
        &vec![
            // File Report
            "WorkID", "4447-System_ItemPathDisplay", "15F-System_DateModified",
            "16F-System_DateCreated", "17F-System_DateAccessed", "13F-System_Size", "4396-System_FileOwner",
            "4625-System_Search_AutoSummary", "14F-System_FileAttributes",
            // IE/Edge History Report
            "4442-System_ItemName", "33-System_ItemUrl", "4468-System_Link_TargetUrl", "4438-System_ItemDate",
            "4470-System_Link_TargetUrlPath",
            // Activity History Report
            "4450-System_ItemType", "4443-System_ItemNameDisplay", "4139-System_ActivityHistory_StartTime",
            "4130-System_ActivityHistory_EndTime", "4137-System_ActivityHistory_LocalStartTime",
            "4136-System_ActivityHistory_LocalEndTime", "4105-System_Activity_AppDisplayName",
            "4123-System_ActivityHistory_AppId", "4115-System_Activity_DisplayText",
            "4112-System_Activity_ContentUri",
        ]
    );
    let mut h = HashMap::new();
    loop {
        let mut workId : u32 = 0;
        for c in &sel_cols {
            if c.name == "WorkID" { // INTEGER
                match get_column::<u32>(&*jdb, table_id, &c) {
                    Ok(r) => match r {
                        Some(wId) => {
                            workId = wId;
                            // Join WorkID within SystemIndex_PropertyStore with DocumentID in SystemIndex_Gthr
                            if let Some(gh) = gather_table_fields.get(&workId) {
                                for (k, v) in gh {
                                    h.insert(k.into(), v.clone());
                                }
                            }
                        },
                        None => {}
                    },
                    Err(e) => println!("Error while getting column {} from {}", c.name, t)
                }
            } else {
                match jdb.get_column(table_id, c.id) {
                    Ok(r) => match r {
                        None => {} //println!("Empty field: {}", c.name),
                        Some(v) => {
                            h.insert(c.name.clone(), v);
                        }
                    },
                    Err(e) => println!("Error while getting column {} from {}", c.name, t)
                }
            }
        }
        ese_dump_file_record(workId, &h);
        ese_IE_history_record(workId, &h);
        ese_activity_history_record(workId, &h);
        h.clear();

        if !jdb.move_row(table_id, ESE_MoveNext)? {
            break;
        }
    }
    Ok(())
}

fn dump(f: &str) -> Result<(), SimpleError> {
    let p = Path::new(f);
    let ext = p.extension().and_then(OsStr::to_str).unwrap();
    if (ext == "edb") {
        dump_file_report_ese(&p)?;
    } else if (ext == "db") {
        dump_file_report_sqlite(&p)?;
    } else {
        return Err(SimpleError::new(format!("Wrong db extension {}, path {}", ext, f)));
    }
    Ok(())
}

fn main() {
    dump("c:/temp/test/Windows.db").unwrap();
}
