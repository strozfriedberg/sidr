# Search Index DB Reporter (SIDR)

SIDR (Search Index DB Reporter) is a Rust-based tool designed to parse Windows search artifacts from Windows 10 (and prior) and Windows 11 systems.
The tool handles both ESE databases (Windows.edb) and SQLite databases (Windows.db) as input and generates three detailed reports as output.

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
          Output report format

          [default: json]
          [possible values: json, csv]

  -r, --report-type <REPORT_TYPE>
          Output results to file or stdout

          [default: to-file]
          [possible values: to-file, to-stdout]

  -o, --outdir <OUTPUT DIRECTORY>
          Path to the directory where reports will be created (will be created if not present). Default is the current directory

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### Example

`> sidr -f json C:\\test`

will scan the C:\test directory for Windows.db and Windows.edb files and will produce 3 logs in the current working directory:
`DESKTOP-12345_File_Report_20230307_015244.json`
`DESKTOP-12345_Internet_History_Report_20230307_015317.json`
`DESKTOP-12345_Activity_History_Report_20230307_015317.json`

Where the filename follows this format:
`HOSTNAME_ReportName_DateTime.json|csv`

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

### Running with cargo

`cargo run --bin sidr -- -f csv --report-type to-file /home/<username>/path/to/tests_s`

### Velociraptor Plugin

The `velosidr.yaml` file can be used to configure a Velociraptor plugin that will run SIDR on a target system.

Note: In order to parse the database on the target endpoint, the SIDR plugin must create an empty database and copy the original database to it. Creating the new copy can overwrite multiple gigabytes of data in unallocated clusters, which may result in loss of evidence. Please use the plugin with caution.

### Copyright

Copyright 2025, LevelBlue. SIDR is licensed under the Apache License, Version 2.0.
