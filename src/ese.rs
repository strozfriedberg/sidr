
use simple_error::SimpleError;
use std::path::Path;
use std::collections::HashMap;

use crate::report::*;
use crate::utils::*;
use crate::fields::*;

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

pub fn ese_generate_report(f: &Path, report_prod: &ReportProducer) -> Result<(), SimpleError> {
    let jdb = Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, f).unwrap());
    let t = "SystemIndex_PropertyStore";
    let table_id = jdb.open_table(t)?;
    let cols = jdb.get_columns(t)?;
    if !jdb.move_row(table_id, ESE_MoveFirst)? {
        // empty table
        return Err(SimpleError::new(format!("Empty table {t}")));
    }
    //let gather_table_fields = dump_file_gather_ese(f)?;

    let (file_rep_path, file_rep) = report_prod.new_report(f, "file-report")?;
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
    file_rep.set_field("4631F-System_Search_GatherTime");
    file_rep.set_field("4450-System_ItemType");

    let (ie_rep_path, ie_rep) = report_prod.new_report(f, "ie-report")?;
    ie_rep.set_field(WORKID);
    ie_rep.set_field(URL);
    ie_rep.set_field(DATE_MODIFIED);
    ie_rep.set_field(FULL_PATH_URL);
    ie_rep.set_field(FULL_PATH_TARGETURL);
    ie_rep.set_field(SYSTEM_TIME_OF_THE_VISIT);
    ie_rep.set_field("4631F-System_Search_GatherTime");

    let (act_rep_path,  act_rep) = report_prod.new_report(f, "act-report")?;
    act_rep.set_field(WORKID);
    act_rep.set_field(ACTIVITYHISTORY_FILENAME);
    act_rep.set_field(ACTIVITYHISTORY_FULLPATH);
    act_rep.set_field(ACTIVITY_START_TIMESTAMP);
    act_rep.set_field(ACTIVITY_END_TIMESTAMP);
    act_rep.set_field(APPLICATION_NAME);
    act_rep.set_field(APPLICATION_GUID);
    act_rep.set_field(ASSOCIATED_FILE);
    act_rep.set_field(VOLUME_ID);
    act_rep.set_field(OBJECT_ID);
    act_rep.set_field(FULLPATH_ASSOCIATED_FILE);

    eprintln!("{}\n{}\n{}\n", file_rep_path.to_string_lossy(), ie_rep_path.to_string_lossy(), act_rep_path.to_string_lossy());

    // prepare to query only selected columns
    let sel_cols = prepare_selected_cols(cols,
        &vec![
            // File Report
            "WorkID", "4447-System_ItemPathDisplay", "15F-System_DateModified",
            "16F-System_DateCreated", "17F-System_DateAccessed", "13F-System_Size", "4396-System_FileOwner",
            "4625-System_Search_AutoSummary", "14F-System_FileAttributes",
            "4631F-System_Search_GatherTime", "4450-System_ItemType",
            // IE/Edge History Report
            "4442-System_ItemName", "33-System_ItemUrl", "4468-System_Link_TargetUrl", "4438-System_ItemDate",
            // Activity History Report
            "4450-System_ItemType", "4443-System_ItemNameDisplay", "4139-System_ActivityHistory_StartTime",
            "4130-System_ActivityHistory_EndTime", "4105-System_Activity_AppDisplayName",
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
        if !ese_IE_history_record(&ie_rep, workId, &h) && !ese_activity_history_record(&act_rep, workId, &h) {
            ese_dump_file_record(&file_rep, workId, &h);
        }
        h.clear();

        if !jdb.move_row(table_id, ESE_MoveNext)? {
            break;
        }
    }
    Ok(())
}

// File Report
fn ese_dump_file_record(r: &Box<dyn Report>, workId: u32, h: &HashMap<String, Vec<u8>>) {
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

    r.new_record();
    r.int_val(WORKID, workId as u64);
    for (col, val) in h {
        match col.as_str() {
            "4447-System_ItemPathDisplay" => r.str_val(col, from_utf16(val)),
            "15F-System_DateModified" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "16F-System_DateCreated" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "17F-System_DateAccessed" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "13F-System_Size" => r.int_val(col, u64::from_bytes(&val)),
            "4396-System_FileOwner" => r.str_val(col, from_utf16(&val)),
            "4625-System_Search_AutoSummary" => r.str_val(col, from_utf16(&val)),
            "14F-System_FileAttributes" => r.str_val(col, file_attributes_to_string(val)),
            "4631F-System_Search_GatherTime" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "4450-System_ItemType" => r.str_val(col, from_utf16(val)),
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

// IE/Edge History Report
fn ese_IE_history_record(r: &Box<dyn Report>, workId: u32, h: &HashMap<String, Vec<u8>>) -> bool {
    if let Some(url_data) = h.get("33-System_ItemUrl") {
        let url = from_utf16(url_data);
        if url.starts_with("iehistory://") {
            r.new_record();
            r.int_val(WORKID, workId as u64);
            for (col, val) in h {
                match col.as_str() {
                    "4442-System_ItemName" => r.str_val(col, from_utf16(val)),
                    "15F-System_DateModified" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
                    "33-System_ItemUrl" => r.str_val(col, url.clone()),
                    "4468-System_Link_TargetUrl" => r.str_val(col, from_utf16(val)),
                    "4438-System_ItemDate" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
                    "4631F-System_Search_GatherTime" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            _ => {}
                }
            }
            return true;
        }
    }
    false
}

// Activity History Report
fn ese_activity_history_record(r: &Box<dyn Report>, workId: u32, h: &HashMap<String, Vec<u8>>) -> bool {
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
    r.new_record();
    r.int_val(WORKID, workId as u64);
    for (col, val) in h {
        match col.as_str() {
            "4443-System_ItemNameDisplay" => r.str_val(col, from_utf16(val)),
            "33-System_ItemUrl" => r.str_val(col, from_utf16(val)), // TODO: get UserSID from here
            "4139-System_ActivityHistory_StartTime" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "4130-System_ActivityHistory_EndTime" => r.str_val(col, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "4105-System_Activity_AppDisplayName" => r.str_val(col, from_utf16(val)),
            "4123-System_ActivityHistory_AppId" => r.str_val(col, from_utf16(val)),
            "4115-System_Activity_DisplayText" => r.str_val(col, from_utf16(val)),
            "4112-System_Activity_ContentUri" => {
                let v = from_utf16(val);
                r.str_val(VOLUME_ID, find_guid(&v, "VolumeId="));
                r.str_val(OBJECT_ID, find_guid(&v, "ObjectId="));
                r.str_val(col, v);
            },
            _ => {}
        }
    }
    true
}