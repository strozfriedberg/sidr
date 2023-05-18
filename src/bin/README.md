# The Windows Search Forensic Artifact Parser with custom configuration

## Configuration
The configuration file uses the [YAML](https://yaml.org/) format:
- first level defines
  - `table_edb` - a table name in `Windows.edb` (or other DB in `esent` format)
  - `table_sql` -  a table name in `Windows.db`  (or other DB in `sqlite` format)
  - `output_format` - default output format (`Csv` or `Json`) (can be changed with `--format` or `-f` command line argument)
  - `output_dir` - default path to generated reports (can be changed with `--outdir` or `-o` command line argument)
- second level (`reports` list) defines titles of requested reports - each report has
  - <a name="rep_title"></a>`title` - report name
  - <a name="rep_constraint"></a>[`constraint`](#constraints) - optional boolean expression
  - `columns` list
    - `title` - name column in report
    - `kind` - column data type (`Integer`, `String`, `DateTime`, `GUID`)
    - <a name="edb_name"></a>`edb.name` - native column name in `esent` DB
    - <a name="edb_constraint"></a>[`constraint`](#constraints) - optional boolean expression to accept [value](#edb_name)
    - <a name="sql_name"></a>`sql.name` - native column name in `sqlite` DB
    - <a name="sql_constraint"></a>[`constraint`](#constraints) - optional boolean expression to accept [value](#sql_name)

## Constraints
There is used [evalexpr](https://docs.rs/evalexpr/latest/evalexpr/) to evaluate expressions.
- on [report level](#rep_constraint) in [Context](https://docs.rs/evalexpr/latest/evalexpr/trait.Context.html) are added boolean variables named as [report name](#rep_title) (`true` - not empty report, `false` - no report was produced).
  For example   
  *<pre>    constraint: "!Internet_History_Report && !Activity_History_Report"</pre>*
- in [edb constraint](#edb_constraint) or [sql constraint](#sql_constraint) there is a list of constraints. 
  In each constraint all literals `{Value}` are replaced with field's value. For example  
  *<pre>    constraint: ['str::regex_matches("{Value}", "^(http://|https://)")']</pre>*
  Besides [evalexpr](https://docs.rs/evalexpr/latest/evalexpr/) there are used some custom flags:
  - `auto_fill` - first not empty value is used for all values. For example
    *<pre>    constraint: [auto_fill]</pre>*
  - `hidden` - field will be not included to report. For example
    *<pre>    constraint: ['str::regex_matches("{Value}", "^ActivityHistoryItem$")',hidden]</pre>*
  - `optional` - assume that the absence of a value satisfies the constraint.

## Application

The application source is in `src/bin/external_cfg.rs`. It accepted command line arguments
- `--cfg_path` or `-c` - path to [configuration](#configuration)
- `--outdir` or `-o` - path to placing output reports
- `--format`or `-f` - output format (`Csv` or `Json`)
- `--db_path` or `-d` - path to the database under study (extension defines format: `.edb` - `esent` format, `.db` - `sqlite` format

## Test
There is a unit test in `tests/gen_reports.rs`. Test parameters are controlled by using environment variables:
- `WSA_TEST_DB_PATH` - corresponds to [Application](#application)'s `--db_path` argument.
- `WSA_TEST_CONFIGURATION_PATH`  - corresponds to [Application](#application)'s `--cfg_path` argument.
- `KEEP_TEMP_WORK_DIR` - optional flag to do not delete temporary directory with test's data.
- `RUST_LOG` - see [env_logger](https://docs.rs/env_logger/latest/env_logger/), all log records are sending on `stderr`.

The test invokes [Application](#application) and `sidr` to produce reports in `csv` and `json` formats 
(`generate_csv_json` function) for `WSA_TEST_DB_PATH`. After that all generated reports are compared. 
