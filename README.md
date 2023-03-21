# Search Index DB Reporter

## About 

SIDR (Search Index DB Reporter) is a tool designed to parse Windows search artifacts from Windows 10 (and prior) and Windows 11 systems. The tool handles both ESE databases (Windows.edb) and SQLite databases (Windows.db) as input and generates three detailed reports as output.

Example:
`> sidr -f json C:\\test`

will scan `C:\test` directory for `Windows.db/Windows.edb` files and produce 3 logs for each database:
`DESKTOP-POG7R45_File_Report_20230307_015244.json`
`DESKTOP-POG7R45_Internet_History_Report_20230307_015317.json`
`DESKTOP-POG7R45_Activity_History_Report_20230307_015317.json`

Where file name of logs consists of:
`HOSTNAME_Report_name_Current_date_and_time.json|csv`

`HOSTNAME` is extracted from the database.