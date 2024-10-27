extern crate exitcode;
use std::process;

use simple_error::SimpleError;
use std::collections::HashMap;
use std::path::Path;

use crate::report::*;
use crate::shared::*;
use crate::utils::*;

use ese_parser_lib::ese_parser::EseParser;
use ese_parser_lib::ese_trait::*;
use std::io::Write;

const CACHE_SIZE_ENTRIES: usize = 10;

fn prepare_selected_cols(cols: Vec<ColumnInfo>, sel_cols: &Vec<&str>) -> Vec<ColumnInfo> {
    let mut only_cols: Vec<ColumnInfo> = Vec::new();
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
                eprintln!("Requested column {i} not found in table columns");
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

pub fn ese_get_hostname(
    jdb: &dyn EseDb,
    table_id: u64,
    columns: &[ColumnInfo],
) -> Result<String, SimpleError> {
    if !jdb.move_row(table_id, ESE_MoveLast)? {
        // empty table
        return Err(SimpleError::new(format!("Empty table {table_id}")));
    }
    let _rollback = DropMe::new(|| {
        let _ = jdb.move_row(table_id, ESE_MoveFirst);
    });
    let sys_comp_name = columns
        .iter()
        .find(|i| column_string_part(&i.name) == "System_ComputerName")
        .ok_or_else(|| SimpleError::new("Can't find field 'System_ComputerName'".to_string()))?;
    let sys_item_type = columns
        .iter()
        .find(|i| column_string_part(&i.name) == "System_ItemType")
        .ok_or_else(|| SimpleError::new("Can't find field 'System_ItemType'".to_string()))?;
    loop {
        if let Some(v) = jdb.get_column(table_id, sys_comp_name.id)? {
            // ASDF-5849
            // accept result only if System_ItemType != ".url"
            match jdb.get_column(table_id, sys_item_type.id)? {
                None => {
                    return Ok(from_utf16(&v));
                }
                Some(v2) => {
                    let item_type = from_utf16(&v2).to_lowercase();
                    if item_type != ".url" {
                        return Ok(from_utf16(&v));
                    }
                }
            }
        }
        if !jdb.move_row(table_id, ESE_MovePrevious)? {
            break;
        }
    }
    Err(SimpleError::new("Empty field System_ComputerName".to_string()))
}

pub fn ese_generate_report(f: &Path, report_prod: &ReportProducer, status_logger: &mut Box<dyn Write>) -> Result<(), SimpleError> {
    writeln!(status_logger, "Processing ESE db: {}", &f.to_string_lossy())
        .map_err(|e| SimpleError::new(format!("{e}")))?;
    let jdb = Box::new(EseParser::load_from_path(CACHE_SIZE_ENTRIES, f).unwrap());
    let edb_database_state = jdb.get_database_state();
    let t = "SystemIndex_PropertyStore";
    let table_id = jdb.open_table(t)?;
    let cols = jdb.get_columns(t)?;
    if !jdb.move_row(table_id, ESE_MoveFirst)? {
        // empty table
        return Err(SimpleError::new(format!("Empty table {t}")));
    }
    //let gather_table_fields = dump_file_gather_ese(f)?;

    // prepare to query only selected columns
    let sel_cols = prepare_selected_cols(
        cols,
        &vec![
            "System_ComputerName",
            "WorkID",
            // File Report
            "System_ItemPathDisplay",
            "System_DateModified",
            "System_DateCreated",
            "System_DateAccessed",
            "System_Size",
            "System_FileOwner",
            "System_Search_AutoSummary",
            "System_Search_GatherTime",
            "System_ItemType",
            // IE/Edge History Report
            "System_ItemUrl",
            "System_Link_TargetUrl",
            "System_ItemDate",
            "System_Title",
            "System_Link_DateVisited",
            // Activity History Report
            "System_ItemNameDisplay",
            "System_ActivityHistory_StartTime",
            "System_ActivityHistory_EndTime",
            "System_Activity_AppDisplayName",
            "System_ActivityHistory_AppId",
            "System_Activity_DisplayText",
            "System_Activity_ContentUri",
        ],
    );

    // get System_ComputerName value
    let recovered_hostname = match ese_get_hostname(&*jdb, table_id, &sel_cols) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("ese_get_hostname() failed: {e}. Will use 'Unknown' as a hostname.");
            "Unknown".to_string()
        }
    };

    let (mut file_rep, mut ie_rep, mut act_rep) =
        init_reports(f, report_prod, &recovered_hostname, status_logger, Some(edb_database_state))?;

    let mut h = HashMap::new();
    loop {
        let mut workId: u32 = 0;
        for c in &sel_cols {
            if c.name == "WorkID" {
                // INTEGER
                match get_column::<u32>(&*jdb, table_id, c) {
                    Ok(r) => {
                        if let Some(wId) = r {
                            workId = wId;
                            // Join WorkID within SystemIndex_PropertyStore with DocumentID in SystemIndex_Gthr
                            // if let Some(gh) = gather_table_fields.get(&workId) {
                            //     for (k, v) in gh {
                            //         h.insert(k.into(), v.clone());
                            //     }
                            // }
                        }
                    }
                    Err(e) => eprintln!("Error while getting column {} from {}: {}", c.name, t, e),
                }
            } else {
                match jdb.get_column(table_id, c.id) {
                    Ok(r) => match r {
                        None => {} //println!("Empty field: {}", c.name),
                        Some(v) => {
                            h.insert(c.name.clone(), v);
                        }
                    },
                    Err(e) => eprintln!("Error while getting column {} from {}: {}", c.name, t, e),
                }
            }
        }
        let ie_history = ese_IE_history_record(&mut *ie_rep, workId, &h);
        let act_history = ese_activity_history_record(&mut *act_rep, workId, &h);
        if !ie_history && !act_history {
            ese_dump_file_record(&mut *file_rep, workId, &h);
        }
        h.clear();

        if !jdb.move_row(table_id, ESE_MoveNext)? {
            break;
        }
    }
    if report_prod.is_db_dirty(Some(edb_database_state)) {
        if report_prod.get_report_type() == ReportOutput::ToStdout {
            eprintln!("WARNING: The database state is not clean");
            process::exit(exitcode::DATAERR)
        }
        else {
            eprintln!("WARNING: The database state is not clean.");
            eprintln!("Processing a dirty database may generate inaccurate and/or incomplete results.\n");
            eprintln!("Use windows\\system32\\esentutl.exe for recovery (/r) and repair (/p).");
            eprintln!("Note that Esentutl must be run from a version of Windows that is equal to or newer than the one that generated the database.");
        }
    }
    Ok(())
}

// File Report
fn ese_dump_file_record(r: &mut dyn Report, workId: u32, h: &HashMap<String, Vec<u8>>) {
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
            "System_DateModified" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_DateCreated" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_DateAccessed" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_Size" => r.int_val(csp, u64::from_bytes(val)),
            "System_FileOwner" => r.str_val(csp, from_utf16(val)),
            "System_Search_AutoSummary" => r.str_val(csp, from_utf16(val)),
            "System_Search_GatherTime" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_ItemType" => r.str_val(csp, from_utf16(val)),
            "System_ComputerName" => r.str_val(csp, from_utf16(val)),
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
fn ese_IE_history_record(r: &mut dyn Report, workId: u32, h: &HashMap<String, Vec<u8>>) -> bool {
    let url = h.get_key_value("33-System_ItemUrl");
    if url.is_none() {
        return false;
    }
    if let Some((_, val)) = url {
        let v = from_utf16(val);
        if !(v.starts_with("iehistory://")
            || v.starts_with("winrt://")
                && v.contains("/LS/Desktop/Microsoft Edge/stable/Default/"))
        {
            return false;
        }
    }

    r.new_record();
    r.int_val("WorkId", workId as u64);
    for (col, val) in h {
        let csp = column_string_part(col);
        match csp {
            "System_DateModified" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_ItemUrl" => r.str_val(csp, from_utf16(val)),
            "System_Link_TargetUrl" => r.str_val(csp, from_utf16(val)),
            "System_ItemDate" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_Search_GatherTime" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_Title" => r.str_val(csp, from_utf16(val)),
            "System_Link_DateVisited" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_ComputerName" => r.str_val(csp, from_utf16(val)),
            _ => {}
        }
    }
    true
}

// Activity History Report
fn ese_activity_history_record(
    r: &mut dyn Report,
    workId: u32,
    h: &HashMap<String, Vec<u8>>,
) -> bool {
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
            "System_ActivityHistory_StartTime" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_ActivityHistory_EndTime" => r.str_val(
                csp,
                format_date_time(get_date_time_from_filetime(u64::from_bytes(val))),
            ),
            "System_Activity_AppDisplayName" => r.str_val(csp, from_utf16(val)),
            "System_ActivityHistory_AppId" => r.str_val(csp, from_utf16(val)),
            "System_Activity_DisplayText" => r.str_val(csp, from_utf16(val)),
            "System_Activity_ContentUri" => {
                let v = from_utf16(val);
                r.str_val("VolumeId", find_guid(&v, "VolumeId="));
                r.str_val("ObjectId", find_guid(&v, "ObjectId="));
                r.str_val(csp, v);
            }
            "System_ComputerName" => r.str_val(csp, from_utf16(val)),
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        fs,
        path::PathBuf,
        process::{Command},
    };
    use tempdir::TempDir;
    use crate::ese::ese_generate_report;
    use simple_error::SimpleError;

    #[test]
    fn warn_dirty() {
        let test_dir =
            TempDir::new("test_warn_dirty").unwrap_or_else(|e| panic!("TempDir::new failed: {e}"));
        let src = "tests/testdata/Windows.edb";
        let dst = test_dir.path().join("Windows.edb");
        fs::copy(src, &dst)
            .unwrap_or_else(|e| panic!("Could not copy '{src}' to '{}': {e}", dst.display()));

        let bin_root = PathBuf::from("target").join(if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        });
        let sidr_bin = bin_root.join("sidr");
        let work_dir = test_dir.path().display().to_string();
        let output = Command::new(sidr_bin)
            .args(["--outdir", &work_dir, &work_dir])
            .output()
            .expect("failed to execute process");

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr)
            .contains("WARNING: The database state is not clean."));
    }
}
