import re
import sys
import yaml

HEADER = """
.load dtformat

ATTACH DATABASE '' AS named;
CREATE TABLE named.NamedFields(
  WorkId INTEGER NOT NULL,
"""

FOOTER = """
.headers on
.mode csv
.output {}_test.csv
select * from named.NamedFields;
.exit
"""

def process(fields_path):
    print(f'process {fields_path}')
    with open(fields_path) as f:
        cfg = yaml.safe_load(f)

    table_sql = cfg['table_sql']
    for report in cfg['reports']:
        views = f'CREATE TEMP VIEW WorkId as SELECT DISTINCT WorkId FROM {table_sql} order by workid;\n'
        sql = 'insert into named.NamedFields(WorkId) SELECT WorkId FROM WorkId;\n'
        header = HEADER

        columns = report['columns']
        for column in columns:
            title = column['title']
            if title == 'WorkId':
                continue

            col_code = column['sql']['name']
            if not col_code:
                continue

            header += f'  {title} STRING,\n'
            sql += f'UPDATE named.NamedFields set {title}=(select {title} from {title} where NamedFields.WorkId = {title}.WorkId);\n'

            template = f'CREATE TEMP VIEW {title} as SELECT a.WorkId, Value as {title} FROM {table_sql} as a inner join WorkId as b on a.WorkId = b.WorkId where a.ColumnId={col_code};\n'
            col_kind = column['kind']
            match col_kind:
                case 'Integer':
                    views += template.replace('Value', 'to_int(Value)')
                case 'DateTime':
                    views += template.replace('Value', 'datetime_format(Value)')
                case 'String':
                    views += template
                case 'GUID':
                    func = f'get_{title}'.lower()
                    views += template.replace('Value', f'{func}(Value)')
                case _:
                    raise Exception(f'"{col_kind}" unexpected')

        header += '  PRIMARY KEY(WorkId)\n);\n'

        report_name = re.sub(r'([\s/]+)', '_', report['title'])
        with open(report_name + '.sql', 'w') as f:
            f.write(header)
            f.write(views)
            f.write(sql)
            f.write(FOOTER.format(report_name))

if __name__ == '__main__':
    if len(sys.argv) != 2:
      sys.exit("Expected path to <configuration.yaml>")

    process(sys.argv[1])