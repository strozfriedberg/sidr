# Search Index DB Reporter (SIDR)

SIDR is a tool designed to parse Windows search artifacts from Windows 10 (and prior) and Windows 11 systems. The tool handles both ESE databases (Windows.edb) and SQLite databases (Windows.db) as input and generates three detailed reports as output.

### Quick Links

* [Usage](#usage)
* [Example](#example)
* [Building](#building)
* [Copyright](#copyright)


### Usage
```
Usage: sidr [OPTIONS] <INPUT>

Arguments:
  <INPUT>
          Path to input directory (which will be recursively scanned for Windows.edb and Windows.db)

Options:
  -f, --format <FORMAT>
          Output format: json (default) or csv
          
          [default: json]
          [possible values: json, csv]

  -o, --outdir <OUTPUT DIRECTORY>
          Path to the directory where reports will be created (will be created if not present). Default is the current directory

  --report-type <REPORT TYPE>
          Output results to file or stdout

          [default: to-file]
          [possible values: to-file, to-stdout]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### Example

`> sidr -f json C:\\test`
`cargo run --bin sidr -- -f csv --report-type to-file /home/<username>/path/to/tests_search_reader` (Linux)

will scan `C:\test` directory for `Windows.db/Windows.edb` files and produce 3 logs for each database:
`DESKTOP-POG7R45_File_Report_20230307_015244.json`
`DESKTOP-POG7R45_Internet_History_Report_20230307_015317.json`
`DESKTOP-POG7R45_Activity_History_Report_20230307_015317.json`

Where file name of logs consists of:
`HOSTNAME_Report_name_Current_date_and_time.json|csv`

`HOSTNAME` is extracted from the database.

### Building

Building SIDR requires [Rust](https://rustup.rs) to be installed. 

To build SIDR:

```
$ git clone https://github.com/strozfriedberg/sidr.git
$ cd sidr
$ cargo build --release
$ ./target/release/sidr --version
sidr 0.8.0
```

### Copyright
Copyright 2023, Aon. SIDR is licensed under the Apache License, Version 2.0.