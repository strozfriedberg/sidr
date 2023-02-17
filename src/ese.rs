
use simple_error::SimpleError;
use std::path::Path;
use std::collections::HashMap;

use crate::utils::*;

use ese_parser_lib::ese_trait::*;
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
    if sel_cols.len() != only_cols.len() {
        for i in sel_cols {
            let mut found = false;
            for j in &only_cols {
                if *i == j.name {
                    found = true;
                    break;
                }
            }
            if !found {
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
/*
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
                    Err(e) => println!("Error while getting column {} from {}: {}", cols[i].name, t, e)
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
*/

pub fn ese_generate_report(f: &Path) -> Result<(), SimpleError> {
    let jdb = Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, f).unwrap());
    let t = "SystemIndex_PropertyStore";
    let table_id = jdb.open_table(t)?;
    let cols = jdb.get_columns(t)?;
    if !jdb.move_row(table_id, ESE_MoveFirst)? {
        // empty table
        return Err(SimpleError::new(format!("Empty table {t}")));
    }
    //let gather_table_fields = dump_file_gather_ese(f)?;

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
                            // if let Some(gh) = gather_table_fields.get(&workId) {
                            //     for (k, v) in gh {
                            //         h.insert(k.into(), v.clone());
                            //     }
                            // }
                        },
                        None => {}
                    },
                    Err(e) => println!("Error while getting column {} from {}: {}", c.name, t, e)
                }
            } else {
                match jdb.get_column(table_id, c.id) {
                    Ok(r) => match r {
                        None => {} //println!("Empty field: {}", c.name),
                        Some(v) => {
                            h.insert(c.name.clone(), v);
                        }
                    },
                    Err(e) => println!("Error while getting column {} from {}: {}", c.name, t, e)
                }
            }
        }
        if !ese_IE_history_record(workId, &h) && !ese_activity_history_record(workId, &h) {
            ese_dump_file_record(workId, &h);
        }
        h.clear();

        if !jdb.move_row(table_id, ESE_MoveNext)? {
            break;
        }
    }
    Ok(())
}

fn ese_dump_file_record(workId: u32, h: &HashMap<String, Vec<u8>>) {
    // let item_type = h.get_key_value("4447-System_ItemPathDisplay");
    // if item_type.is_none() {
    //     return ;
    // }
    // if let Some((_, val)) = item_type {
    //     let v = from_utf16(val);
    //     if !v[1..].starts_with(":\\") {
    //         eprintln!("workId {} Full Path {}", workId, v);
    //         //return ;
    //     }
    // }
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
            "14F-System_FileAttributes" => println!("File Attributes: {:#04X?}", u32::from_bytes(val)), // TODO: pretty print? E.g. FILE_ATTRIBUTE_READONLY, etc.
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

fn ese_IE_history_record(workId: u32, h: &HashMap<String, Vec<u8>>) -> bool {
    if let Some(url_data) = h.get("33-System_ItemUrl") {
        let url = from_utf16(url_data);
        if url.starts_with("iehistory://") {
            println!("IE/Edge History Report for WorkId {}", workId);
            for (col, val) in h {
                match col.as_str() {
                    "4442-System_ItemName" => println!("URL: {}", from_utf16(val)),
                    "4447-System_ItemPathDisplay" => println!("URL(ItemPathDisplay): {}", from_utf16(val)),
                    "15F-System_DateModified" => println!("Modified time: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
                    "33-System_ItemUrl" => println!("Full Path of the URL: {}", url),
                    "4468-System_Link_TargetUrl" => println!("Full Path of the URL (TargetUrl): {}", from_utf16(val)),
                    "4438-System_ItemDate" => println!("System Time of the visit: {}", format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
                    "4470-System_Link_TargetUrlPath" => println!("TargetUrl: {}", from_utf16(val)),
                    _ => {}
                }
            }
            println!("");
            return true;
        }
    }
    false
}

fn ese_activity_history_record(workId: u32, h: &HashMap<String, Vec<u8>>) -> bool {
    // record only if "4450-System_ItemType" == "ActivityHistoryItem"
    let item_type = h.get_key_value("4450-System_ItemType");
    if item_type.is_none() {
        return false;
    }
    if let Some((_, val)) = item_type {
        let v = from_utf16(val);
        if v != "ActivityHistoryItem" {
            return false;
        }
    }
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
            "4112-System_Activity_ContentUri" => {
                let v = from_utf16(val);
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