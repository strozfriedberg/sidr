import csv
import json
import sys
import os.path
import glob

def process(source):
    with open(source, 'r') as f:
        jsonl_content = f.read()

    data = [json.loads(jline.strip()) for jline in jsonl_content.splitlines()]
    headers = data[0].keys()

    parts = os.path.split(source)
    output = os.path.join(parts[0], os.path.splitext(parts[1])[0] + '_json.csv')

    with open(output, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=headers)
        writer.writeheader()
        writer.writerows(data)

if __name__ == '__main__':
    work_dir = '/.'
    if len(sys.argv) == 2:
        work_dir = sys.argv[1]

    for source in glob.glob('*.json'):
      process(source)