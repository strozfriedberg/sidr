import re
import sys
import yaml

BODY = """
.import {report_name}.csv {report_name} --csv
.import {report_name}_ese.csv {report_name}_ese_csv --csv
.import {report_name}_json.csv {report_name}_ese_json --csv

create table diffs_ese_csv as select * from (SELECT * FROM {report_name} EXCEPT SELECT * FROM {report_name}_ese_csv) union select * from (SELECT * FROM {report_name}_ese_csv EXCEPT SELECT * FROM {report_name});
.once {report_name}_ese_csv.discrepancy
select count(*) from diffs_ese_csv;

create table diffs_ese_json as select * from (SELECT * FROM {report_name} EXCEPT SELECT * FROM {report_name}_ese_json) union select * from (SELECT * FROM {report_name}_ese_json EXCEPT SELECT * FROM {report_name});
.once {report_name}_ese_json.discrepancy
select count(*) from diffs_ese_json;
"""


def process(config_path):
    print(f'{sys.argv[0]}: process {config_path}')
    with open(config_path) as f:
        cfg = yaml.safe_load(f)

    for report in cfg['reports']:
        report_name = re.sub(r'([\s/]+)', '_', report['title'])
        with open(report_name + '.sql', 'w') as f:
            f.write(BODY.replace("{report_name}", report_name))


if __name__ == '__main__':
    if len(sys.argv) != 2:
      sys.exit("Expected path to <configuration.yaml>")

    process(sys.argv[1])
