# The Windows Search Forensic Artifact Parser with custom configuration

## Configuration
The configuration file uses the [YAML](https://yaml.org/) format:
- first level defines
-- `table_edb` - a table name in `Windows.edb` (or other DB in `esent` format)
-- `table_sql` -  a table name in `Windows.db`  (or other DB in `sqlite` format)
-- `output_format` - default output format (`Csv` or `Json`) (can be changed with `--format` or `-f` command line argument)
-- `output_dir` - default path to generated reports (can be changed with `--outdir` or `-o` command line argument)
- second level (`reports` list) defines titles of requested reports
- each report has
-- `title` - report name
-- `columns` list
--- `title` - name column in report
--- `kind` - column data type (`Integer`, `String`, `DateTime`, `GUID`)
--- `edb.name` - native column nave in `esent` DB
--- `sql.name` - native column nave in `sqlite` DB

## Application

The application source is in `src/bin/external_cfg.rs`. It accepted command line arguments
- `--cfg_path` or `-c` - path to [configuration](#Configuration)
- `--outdir` or `-o` - path to placing output reports
- `--format`or `-f` - output format (`Csv` or `Json`)
- `--db_path` or `-d` - path to the database under study (extension defines format: `.edb` - `esent` format, `.db` - `sqlite` format

## Test
There is an unit test in `tests/gen_reports.rs`. Test parameters are controlled using environment variables:
- `WSA_TEST_WINDOWS_DB_PATH` - corresponds to [Application](#Application)'s `--db_path` argument in `sqlite` format case
- `WSA_TEST_WINDOWS_EDB_PATH`  - corresponds to [Application](#Application)'s `--db_path` argument in `esent` format case
- `WSA_TEST_CONFIGURATION_PATH`  - corresponds to [Application](#Application)'s `--cfg_path` argument
- `WSA_TEST_PYTHON_SCRIPTS_PATH` - path to directory with `sql_2_csv.py`, `json_2_csv.py` and `ese_2_csv.py` scripts
- `ENV_SQLITE3EXT_H_PATH` - path to directory with `sqlite3ext.h` header (required to build `dtformat` sqlite extension)
- `KEEP_TEMP_WORK_DIR` - optional flag to do not delete temporary directory with test's data

The test invokes [Application](#Application) to produce reports in `csv` and `json` formats (`generate_csv_json` function) for `WSA_TEST_WINDOWS_DB_PATH` (in `do_sql_test` function) and `WSA_TEST_WINDOWS_EDB_PATH` (in `do_ese_test` function).
- for `WSA_TEST_WINDOWS_DB_PATH`:
-- builds `dtformat`sqlite extension from `dtformat.c`
-- convets `json` reports to `csv` with `json_2_csv.py`
-- generates (using `sql_2_csv.py`) scripts for `sqlite3` shell  to build `tables` and `selects` for produce data to compare with [Application](#Application)'s reports. These scripts imports [Application](#Application)'s reports from `.csv` into in-memory tables and do comparison with
`SELECT * FROM {report_name} EXCEPT SELECT * FROM {report_name}_json) union select * from (SELECT * FROM {report_name}_json EXCEPT SELECT * FROM {report_name}` and store count of differences in `.discrepancy` files)
- for `WSA_TEST_WINDOWS_EDB_PATH`:
-- with help of [EseAPI](https://stash.strozfriedberg.com/projects/ASDF/repos/ese_parser/browse/lib/src/esent/ese_api.rs) `do_ese_test` function builds `..._ese.csv` datasets
-- convets `json` reports to `csv` with `json_2_csv.py`
-- generates (using `ese_2_csv.py`) scripts for `sqlite3` shell  similar `sqlite` case
- `compare_with_sql_select` function checks all `.discrepancy` files to have only `0` otherwise test failed
