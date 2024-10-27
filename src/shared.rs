use simple_error::SimpleError;
use std::path::Path;

use crate::report::*;
use ese_parser_lib::parser::jet::DbState;
use std::io::Write;

type Reports = (
    Box<dyn Report>, /* file report */
    Box<dyn Report>, /* ie report */
    Box<dyn Report>, /* act report */
);

pub fn init_reports(
    f: &Path,
    report_prod: &ReportProducer,
    recovered_hostname: &str,
    status_logger: &mut Box<dyn Write>,
    edb_database_state: Option<DbState>,
) -> Result<Reports, SimpleError> {
    let (file_rep_path, file_rep) =
        report_prod.new_report(f, recovered_hostname, "File_Report", edb_database_state)?;

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

    let (ie_rep_path, ie_rep) = report_prod.new_report(
        f,
        recovered_hostname,
        "Internet_History_Report",
        edb_database_state,
    )?;
    ie_rep.set_field("WorkId");
    ie_rep.set_field("System_ComputerName");
    ie_rep.set_field("System_ItemName");
    ie_rep.set_field("System_ItemUrl");
    ie_rep.set_field("System_Link_TargetUrl");
    ie_rep.set_field("System_ItemDate");
    ie_rep.set_field("System_DateCreated");
    ie_rep.set_field("System_DateModified");
    ie_rep.set_field("System_ItemFolderNameDisplay");
    ie_rep.set_field("System_Search_GatherTime");
    ie_rep.set_field("System_Title");
    ie_rep.set_field("System_Link_DateVisited");

    let (act_rep_path, act_rep) = report_prod.new_report(
        f,
        recovered_hostname,
        "Activity_History_Report",
        edb_database_state,
    )?;
    act_rep.set_field("WorkId");
    act_rep.set_field("System_ComputerName");
    act_rep.set_field("System_DateModified");
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

    writeln!(
        status_logger,
        "{}\n{}\n{}\n",
        file_rep_path.to_string_lossy(),
        ie_rep_path.to_string_lossy(),
        act_rep_path.to_string_lossy()
    )
    .map_err(|e| SimpleError::new(format!("{e}")))?;
    Ok((file_rep, ie_rep, act_rep))
}
