
use simple_error::SimpleError;
use std::path::Path;
use std::collections::HashMap;

use crate::report::*;
use crate::utils::*;

use ese_parser_lib::ese_trait::*;
use ese_parser_lib::ese_parser::EseParser;

const CACHE_SIZE_ENTRIES: usize = 10;

fn prepare_selected_cols(cols: Vec<ColumnInfo>, sel_cols: &Vec<&str>) -> Vec<ColumnInfo> {
    let mut only_cols : Vec<ColumnInfo> = Vec::new();
    for c in cols {
        for sc in sel_cols {
            if *sc == column_string_part(&c.name) {
                only_cols.push(c);
                break;
            }
        }
    }
    if sel_cols.len() != only_cols.len() {
        for i in sel_cols {
            let mut found = false;
            for j in &only_cols {
                if *i == column_string_part(&j.name) {
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
    file_rep.set_field("WorkId");
    file_rep.set_field("System_ItemPathDisplay");
    file_rep.set_field("System_DateModified");
    file_rep.set_field("System_DateCreated");
    file_rep.set_field("System_DateAccessed");
    file_rep.set_field("System_Size");
    file_rep.set_field("System_FileOwner");
    file_rep.set_field("System_Search_AutoSummary");
    file_rep.set_field("System_FileAttributes");
    file_rep.set_field("System_Search_GatherTime");
    file_rep.set_field("System_ItemType");

    let (ie_rep_path, ie_rep) = report_prod.new_report(f, "ie-report")?;
    ie_rep.set_field("WorkId");
    ie_rep.set_field("System_ItemName");
    ie_rep.set_field("System_DateModified");
    ie_rep.set_field("System_ItemUrl");
    ie_rep.set_field("System_Link_TargetUrl");
    ie_rep.set_field("System_ItemDate");
    ie_rep.set_field("System_Search_GatherTime");

    let (act_rep_path,  act_rep) = report_prod.new_report(f, "act-report")?;
    act_rep.set_field("WorkId");
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

    // prepare to query only selected columns
    let sel_cols = prepare_selected_cols(cols,
        &vec![
            // File Report
            "WorkID", "System_ItemPathDisplay", "System_DateModified",
            "System_DateCreated", "System_DateAccessed", "System_Size", "System_FileOwner",
            "System_Search_AutoSummary", "System_FileAttributes",
            "System_Search_GatherTime", "System_ItemType",
            // IE/Edge History Report
            "System_ItemName", "System_ItemUrl", "System_Link_TargetUrl", "System_ItemDate",
            // Activity History Report
            "System_ItemType", "System_ItemNameDisplay", "System_ActivityHistory_StartTime",
            "System_ActivityHistory_EndTime", "System_Activity_AppDisplayName",
            "System_ActivityHistory_AppId", "System_Activity_DisplayText",
            "System_Activity_ContentUri",
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
    r.int_val("WorkId", workId as u64);
    for (col, val) in h {
        let csp = column_string_part(col);
        match csp {
            "System_ItemPathDisplay" => r.str_val(csp, from_utf16(val)),
            "System_DateModified" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "System_DateCreated" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "System_DateAccessed" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "System_Size" => r.int_val(csp, u64::from_bytes(&val)),
            "System_FileOwner" => r.str_val(csp, from_utf16(&val)),
            "System_Search_AutoSummary" => r.str_val(csp, from_utf16(&val)),
            "System_FileAttributes" => r.str_val(csp, file_attributes_to_string(val)),
            "System_Search_GatherTime" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "System_ItemType" => r.str_val(csp, from_utf16(val)),
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
            r.int_val("WorkId", workId as u64);
            for (col, val) in h {
                let csp = column_string_part(col);
                match csp {
                    "System_ItemName" => r.str_val(csp, from_utf16(val)),
                    "System_DateModified" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
                    "System_ItemUrl" => r.str_val(csp, url.clone()),
                    "System_Link_TargetUrl" => r.str_val(csp, from_utf16(val)),
                    "System_ItemDate" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
                    "System_Search_GatherTime" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
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
    r.int_val("WorkId", workId as u64);
    for (col, val) in h {
        let csp = column_string_part(col);
        match csp {
            "System_ItemNameDisplay" => r.str_val(csp, from_utf16(val)),
            "System_ItemUrl" => r.str_val(csp, from_utf16(val)), // TODO: get UserSID from here
            "System_ActivityHistory_StartTime" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "System_ActivityHistory_EndTime" => r.str_val(csp, format_date_time(get_date_time_from_filetime(u64::from_bytes(&val)))),
            "System_Activity_AppDisplayName" => r.str_val(csp, from_utf16(val)),
            "System_ActivityHistory_AppId" => r.str_val(csp, from_utf16(val)),
            "System_Activity_DisplayText" => r.str_val(csp, from_utf16(val)),
            "System_Activity_ContentUri" => {
                let v = from_utf16(val);
                r.str_val("VolumeId", find_guid(&v, "VolumeId="));
                r.str_val("ObjectId", find_guid(&v, "ObjectId="));
                r.str_val(csp, v);
            },
            _ => {}
        }
    }
    true
}