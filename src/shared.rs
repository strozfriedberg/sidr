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

    let (ie_rep_path, ie_rep) = report_prod.new_report(
        f,
        recovered_hostname,
        "Internet_History_Report",
        edb_database_state,
    )?;

    let (act_rep_path, act_rep) = report_prod.new_report(
        f,
        recovered_hostname,
        "Activity_History_Report",
        edb_database_state,
    )?;

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
