# The Windows Search Forensic Artifact Parser with custom configuration

## Introduction
`external_cfg` app is used for more control of database parsing. See `Configuration` section about setting up
the config file. Examples of configs can be found in `src/bin`.
The app shares code with sidr. For example, `ReportProducer::new` is called in both.

## Configuration
The configuration file uses the [YAML](https://yaml.org/) format:
- first level defines
  - `table_edb` - a table name in `Windows.edb` (or other DB in [ESENT](https://github.com/libyal/libesedb/blob/main/documentation/Extensible%20Storage%20Engine%20(ESE)%20Database%20File%20(EDB)%20format.asciidoc) format)
  - `table_sql` -  a table name in `Windows.db`  (or other DB in `sqlite` format)
  - `output_format` - default output format (`Csv` or `Json`) (can be changed with `--format` or `-f` command line argument)
  - `output_type` - output results to file or stdout (`ToFile` or `ToStdout`)
  - `output_dir` - default path to generated reports (can be changed with `--outdir` or `-o` command line argument)
- second level (`reports` list) defines titles of requested reports - each report has
  - <a name="rep_title"></a>`title` - report name
  - <a name="rep_constraint"></a>[`constraint`](#constraints) - optional boolean expression
  - `columns` list
    - `title` - name of column in report
    - `kind` - column data type (`Integer`, `String`, `DateTime`, `GUID`)
    - <a name="edb_name"></a>`edb.name` - native column name in `ESENT` DB. Short form is acceptable (e.g., `System_ComputerName` is a short form of `4184-System_ComputerName`)
    - <a name="edb_constraint"></a>[`constraint`](#constraints) - optional boolean expression to accept [value](#edb_name)
    - <a name="sql_name"></a>`sql.name` - native column name in `sqlite` DB
    - <a name="sql_constraint"></a>[`constraint`](#constraints) - optional boolean expression to accept [value](#sql_name)

## Configuration overrides
Some config values can be overridden in `external_cfg.rs`. For example:
```
    // Override config.yml with cli.
    // cfg.output_type = match cli.report_type {
    //     ReportType::ToFile => wsa_lib::OutputType::ToFile,
    //     ReportType::ToStdout => wsa_lib::OutputType::ToStdout,
    // };
```
Cli `report_type` will be used instead of the config `output_type` value.

## Constraints
SIDR uses [evalexpr](https://docs.rs/evalexpr/latest/evalexpr/) to evaluate expressions.
- On the [report level](#rep_constraint) in the [Context](https://docs.rs/evalexpr/latest/evalexpr/trait.Context.html) trait, we have added a boolean variable that indicates the status of the report. The name of this variable will match the name of the report. A `true` value indicates a non-empty report, while a `false` value indicates that SIDR did not produce that report.
  * For example:
  *<pre>constraint: "!Internet_History_Report && !Activity_History_Report"</pre>*
- Both the [edb constraint](#edb_constraint) and [sql constraint](#sql_constraint) contain a list of constraints. 
  In each constraint, all `{Value}` literals are replaced with the field's value. 
  For example:
  *<pre>constraint: ['str::regex_matches("{Value}", "^(http://|https://)")']</pre>*
  * Besides [evalexpr](https://docs.rs/evalexpr/latest/evalexpr/), the configuration may include the following custom flags:
    - `auto_fill` - indicates that the first non-empty value will be used for all values. 
  For example:
    *<pre>constraint: [auto_fill]</pre>*
    - `hidden` - indicates that the field will be not included in the report. For example:
    *<pre>constraint: ['str::regex_matches("{Value}", "^ActivityHistoryItem$")',hidden]</pre>*
  - `optional` - indicates that the absence of a value satisfies the constraint.

## Application

The application source is in `src/bin/external_cfg.rs`. It accepts the folllowing command line arguments:

- `--cfg_path` or `-c` path to [configuration](#configuration)
- `--outdir` or `-o` - path to directory where reports will be saved
- `--format` or `-f` - output format (`csv` or `json`)
- `--db_path` or `-d` - path to the Windows Search Database
  - Extension indicates DB format (i.e., `.edb` indicates `ESENT` format, while `.db` indicates `sqlite` format)
                        Important: the option points to the folder where the db files are (not to the file itself)
- `--report-type` - output results to file or stdout. Default: `to-file`. Possible values: `to-file`, `to-stdout`

Example:
`cargo run --bin external_cfg -- -f csv --report-type to-stdout -c /path/to/windows_search_artifact/src/bin/test_reports_cfg.yaml /path/to/tests_search_reader`


## Test
There is a unit test in `tests/gen_reports.rs`. Test parameters are controlled by using environment variables:
- `WSA_TEST_DB_PATH` - corresponds to [Application](#application)'s `--db_path` argument.
                       Important: the var points to the folder where the db files are (not to the file itself)
- `WSA_TEST_CONFIGURATION_PATH`  - corresponds to [Application](#application)'s `--cfg_path` argument.
- `KEEP_TEMP_WORK_DIR` - optional flag to do not delete temporary directory with test's data.
- `RUST_LOG` - see [env_logger](https://docs.rs/env_logger/latest/env_logger/), all log records are sending on `stderr`.

The test invokes [Application](#application) and `sidr` to produce reports in `csv` and `json` formats 
(`generate_csv_json` function) for `WSA_TEST_DB_PATH`. After that all generated reports are compared. 
