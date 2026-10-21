"""Independent offline audit of completed native run; never writes its inputs."""
import collections
import hashlib
import json
import re
import subprocess
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET
import openpyxl

own = Path(__file__).resolve().parent
root = own.parent.parent
native = root / 'artifacts/table-xlsx-native-run-20261021-r1/native'
digest = lambda value: hashlib.sha256(value).hexdigest()
evidence_file = native / 'ui-table-xlsx-live.json'
evidence = json.loads(evidence_file.read_text(encoding='utf-8'))
assert evidence['result'] == 'PASS', 'Original native result is not PASS'
assert evidence['binary_sha256'] == 'c2df6f1ce6699b8c91af9488b8c71a77f91b5991a1df9b7c1f6f76cb2a4aa72c'
assert len(evidence['workbooks']) == 30
labels = dict(url='URL', title_text='Seitentitel', title_length='Zeichen', title_pixels='Pixelprofil 20px', title_count='Title-Tags', duplicates='Vorkommen', status='HTTP final', initial_status='HTTP initial', final_url='Finale URL', redirects='Redirects', response_ms='Antwortzeit', indexable='Indexierbarkeit', error='Abruffehler', requested_url='Anfrage-URL', response_index='Antwortposition')
metadata = ['Crawl Coverage', 'Observation Basis', 'Selected State', 'Parent Snapshot SHA256', 'Fetched At', 'Observation Requested URL']
title_keys = ['url', 'title_text', 'title_length', 'title_pixels', 'title_count', 'duplicates', 'status']
status_keys = ['url', 'initial_status', 'status', 'final_url', 'redirects', 'response_ms', 'indexable', 'error']
main_keys = ['url', 'title_count', 'title_text', 'title_pixels', 'status']
response_keys = ['url', 'requested_url', 'response_index', 'status']
parts = ['[Content_Types].xml', '_rels/.rels', 'xl/workbook.xml', 'xl/_rels/workbook.xml.rels', 'xl/worksheets/sheet1.xml']
ns = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
decode = lambda text: re.sub(r'_x([0-9a-fA-F]{4})_', lambda match: chr(int(match[1], 16)), text)
def kind(value):
    if value is None: return 'null'
    if isinstance(value, str): return 'string'
    if isinstance(value, bool): return 'boolean'
    if isinstance(value, (int, float)): return 'number'
    raise AssertionError('Unsupported audit scalar')
def col(index):
    value = ''
    while index:
        index, rem = divmod(index - 1, 26)
        value = chr(65 + rem) + value
    return value
def same(actual, wanted):
    return kind(actual) == kind(wanted) and actual == wanted

pinned = {}
def read(file):
    value = file.read_bytes()
    prior = pinned.setdefault(str(file.relative_to(root)), digest(value))
    assert prior == digest(value), 'Input changed during audit: ' + str(file)
    return value
read(evidence_file)
captures = {}
for capture in evidence['native_captures']:
    report_file = native / (capture['id'] + '-original-project.json')
    raw = read(report_file)
    assert digest(raw) == capture['project_sha256']
    report = json.loads(raw)
    assert len(report['pages']) == capture['pages']
    saved = read(native / 'store' / (capture['id'] + '.json'))
    assert digest(saved) == capture['saved_sha256']
    assert json.loads(saved) == report, 'Original native store/report semantics changed'
    captures[digest(raw)] = report

# Use the platform standard ICU collation independently, without importing the
# product or native test code. Python reconstructs all populations and scalars.
texts = list({(page.get('titles') or [''])[0] for report in captures.values() for page in report['pages']})
collation = subprocess.run(['node', '-e', "const fs=require('node:fs');const values=JSON.parse(fs.readFileSync(0,'utf8'));const c=new Intl.Collator('de-DE',{numeric:true,sensitivity:'base'});process.stdout.write(JSON.stringify(values.sort((a,b)=>c.compare(a.normalize('NFC'),b.normalize('NFC')))));"], input=json.dumps(texts), text=True, encoding='utf-8', capture_output=True, check=True)
ordered_texts = json.loads(collation.stdout)
title_rank = {value: index for index, value in enumerate(ordered_texts)}
def titles(report):
    pages = [page for page in report['pages'] if page['internal'] and page['content_type'].split(';')[0] in ['text/html', 'application/xhtml+xml']]
    counts = collections.Counter((page.get('titles') or [''])[0].strip() for page in pages if (page.get('titles') or [''])[0].strip())
    return [dict(url=page['url'], title_text=page['titles'][0] if page['titles'] else None, title_length=len((page.get('titles') or [''])[0]), title_pixels=page['title_pixel_widths'][0] if page['title_pixel_widths'] else None, title_count=len(page['titles']), duplicates=counts.get((page.get('titles') or [''])[0].strip(), 0), status=page['final_status'] if page['final_status'] is not None else page['status'], page=page) for page in pages]
def statuses(report):
    return [dict(url=p['url'], initial_status=p['status'], status=p['final_status'] if p['final_status'] is not None else p['status'], final_url=p['final_url'], redirects=len(p['redirect_chain']), response_ms=p['response_ms'], indexable=None if p['indexable'] is None else 'Indexierbar' if p['indexable'] else 'Nicht indexierbar', error=p['error'], page=p) for p in report['pages'] if p['internal']]
def responses(report):
    result = []
    for page in report['pages']:
        if page['error']:
            assert page['status'] is None and not page['redirect_chain']
            continue
        observations = [(hop['url'], hop['status']) for hop in page['redirect_chain']]
        observations.append((page['final_url'] or page['url'], page['final_status'] if page['final_status'] is not None else page['status']))
        for index, (url, status) in enumerate(observations, 1):
            result.append(dict(url=url, requested_url=page['url'], response_index=index, status=status, page=page))
    return sorted(result, key=lambda row: (row['url'], row['requested_url'], row['response_index']))
def population(label, report):
    if 'empty0' in label: return 'titles', main_keys, []
    if 'titles152' in label: return 'titles', title_keys, sorted(titles(report), key=lambda row: row['url'])
    if 'status153' in label: return 'status', status_keys, sorted(statuses(report), key=lambda row: row['url'])
    if 'response' in label:
        rows = responses(report)
        if 'twins2' in label: rows = [row for row in rows if row['url'].endswith('/items/0149')]
        return 'responses', response_keys, rows
    assert any(value in label for value in ['v1-118', 'v2-118', 'responsive']), label
    rows = []
    for row in titles(report):
        match = re.search(r'/items/(\d{4})$', row['url'])
        if match:
            index = int(match[1]); observed = 149 if index == 150 else index
            if observed >= 4 and observed % 5 != 0: rows.append(row)
    assert len(rows) == 118
    return 'titles', main_keys, sorted(rows, key=lambda row: (-title_rank[row['title_text']], row['url']))

files, total_cells, general_cells, protected_cells = [], 0, 0, 0
for workbook in evidence['workbooks']:
    label = workbook['label']; xlsx = native / (label + '.xlsx')
    raw = read(xlsx); assert digest(raw) == workbook['sha256']
    expected = json.loads(read(native / (label + '.independent-expected.json')))
    report = captures[expected['project_sha256']]
    section, keys, rows = population(label, report)
    assert expected['section'] == section and expected['keys'] == keys
    header = [('HTTP beobachtet' if section == 'responses' and key == 'status' else labels[key]) for key in keys] + metadata
    reconstructed = []
    for row in rows:
        provenance = ['crawl_snapshot', 'unknown_or_mixed_row_identity', '', '', '', ''] if section == 'responses' else ['crawl_snapshot', 'captured', '', '', row['page']['fetched_at'], row['page']['url']]
        reconstructed.append([row.get(key) for key in keys] + provenance)
    assert expected['header'] == header and expected['rows'] == reconstructed, label + ' native expectation differs from reviewer reconstruction'
    matrix = [header] + reconstructed
    with zipfile.ZipFile(xlsx) as package:
        assert package.testzip() is None and package.namelist() == parts
        documents = {name: ET.fromstring(package.read(name)) for name in parts}
        sheet = documents['xl/worksheets/sheet1.xml']
        assert sheet.find(ns + 'dimension').attrib == {'ref': 'A1:' + col(len(header)) + str(len(matrix))}
        physical = sheet.find(ns + 'sheetData')
        assert len(physical) == len(matrix)
        kinds = []
        for row_index, (physical_row, wanted) in enumerate(zip(physical, matrix), 1):
            assert physical_row.attrib == {'r': str(row_index)} and len(physical_row) == len(wanted)
            row_kinds = []
            for column_index, (cell, want) in enumerate(zip(physical_row, wanted), 1):
                address = col(column_index) + str(row_index); wire_type = cell.attrib.get('t')
                assert cell.tag == ns + 'c' and cell.attrib.get('r') == address
                if wire_type is None:
                    assert cell.attrib == {'r': address} and not len(cell) and cell.text is None
                    value = None
                elif wire_type == 'inlineStr':
                    assert cell.attrib == {'r': address, 't': 'inlineStr'} and len(cell) == 1
                    inline = cell[0]; assert inline.tag == ns + 'is' and not inline.attrib and len(inline) == 1
                    text = inline[0]; assert text.tag == ns + 't' and not len(text) and text.attrib == {'{http://www.w3.org/XML/1998/namespace}space': 'preserve'}
                    value = decode(text.text or '')
                elif wire_type in ['n', 'b']:
                    assert cell.attrib == {'r': address, 't': wire_type} and len(cell) == 1 and cell[0].tag == ns + 'v' and not len(cell[0])
                    lexical = cell[0].text
                    if wire_type == 'b':
                        assert lexical in ['0', '1']; value = lexical == '1'
                    else: value = float(lexical)
                else: raise AssertionError(label + ' unexpected cell type at ' + address)
                assert same(value, want), (label, address, value, want)
                row_kinds.append(kind(value)); total_cells += 1
            kinds.append(row_kinds)
    third_party = openpyxl.load_workbook(xlsx, data_only=False)
    assert third_party.sheetnames == ['Ergebnisse']
    observed = list(third_party.active.rows)
    assert len(observed) == len(matrix)
    raw_compared, xstring_compared = 0, 0
    for got_row, wanted in zip(observed, matrix):
        assert len(got_row) == len(wanted)
        for cell, want in zip(got_row, wanted):
            assert cell.data_type == {'null': 'n', 'string': 's', 'number': 'n', 'boolean': 'b'}[kind(want)]
            if same(cell.value, want): raw_compared += 1
            else:
                assert isinstance(cell.value, str) and isinstance(want, str) and decode(cell.value) == want
                xstring_compared += 1
    general_cells += raw_compared; protected_cells += xstring_compared
    files.append(dict(label=label, sha256=digest(raw), section=section, rows=len(reconstructed), columns=len(header), checked_cells=len(matrix)*len(header), openpyxl_raw_cells=raw_compared, openpyxl_standards_xstring_adapter_cells=xstring_compared, source_report_sha256=expected['project_sha256'], zip_crc='PASS', standard_xml='PASS', all_cells_types_order_provenance='PASS'))

negative = native / 'declared-negative-corrupt-crc.xlsx'
corrupt = read(negative)
try:
    with zipfile.ZipFile(negative) as package:
        assert package.testzip() is not None
except zipfile.BadZipFile:
    pass
for relative, before in pinned.items(): assert digest((root / relative).read_bytes()) == before, 'Original evidence changed during review'
result = dict(result='PASS', original_native_result=evidence['result'], original_native_binary_sha256=evidence['binary_sha256'], original_evidence_sha256=pinned[str(evidence_file.relative_to(root))], audited_workbooks=len(files), reconstructed_data_rows=sum(item['rows'] for item in files), physical_cells_checked=total_cells, openpyxl_version=openpyxl.__version__, openpyxl_raw_cells_compared=general_cells, openpyxl_standards_xstring_adapter_cells_compared=protected_cells, declared_negative_crc_rejected=True, original_inputs_unchanged=True, workbooks=files, pinned_inputs=pinned, scope='Offline actual-file audit only. All data/header/type/order/provenance reconstructed from immutable original native reports and declared fixture populations; standard Node ICU only for German title collation. No product/planner/compiler/writer or original reader imports. Standard Python ZIP/XML and openpyxl parse all actual files. One-pass standards Xstring adaptation compensates for openpyxl inline-text reader gap; actual Excel/LibreOffice GUI repair/save and unsupported main-table/specialist paths are not proven.')
(own / 'audit.json').write_text(json.dumps(result, indent=2, ensure_ascii=True) + '\n', encoding='utf-8')
print(json.dumps({key: value for key, value in result.items() if key not in ['workbooks', 'pinned_inputs']}, indent=2))
