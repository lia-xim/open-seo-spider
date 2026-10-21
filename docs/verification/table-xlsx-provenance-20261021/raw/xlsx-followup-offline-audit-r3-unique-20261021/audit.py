"""Independent offline source reconstruction and original downloaded ZIP/XML/CSV audit.

No product, native test oracle, expected-matrix, browser, network or native imports.
Only completed final receipts are consumed; original FAIL remains FAIL/PARTIAL.
"""
import argparse
import collections
import csv
import decimal
import hashlib
import io
import json
import re
import struct
import sys
import zipfile
from pathlib import Path
from urllib.parse import urlsplit, urldefrag
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
NS = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
PARTS = ['[Content_Types].xml', '_rels/.rels', 'xl/workbook.xml', 'xl/_rels/workbook.xml.rels', 'xl/worksheets/sheet1.xml']
PROVENANCE = ['Crawl Coverage', 'Observation Basis', 'Selected State', 'Parent Snapshot SHA256', 'Fetched At', 'Observation Requested URL']
SEGMENT = ['Segment Snapshot Token', 'Segment Rules SHA256', 'Segment Typed Report SHA256', 'Segment Generation', 'Segment Selection', 'Segment Reporting Identity', 'Original Page Ordinal']
REPORTING = ['Captured Pages', 'Visible Pages', 'Hidden Pages']
LABELS = dict(url='URL', title_text='Seitentitel', title_length='Zeichen', title_pixels='Pixelprofil 20px', title_count='Title-Tags', duplicates='Vorkommen', status='HTTP final', initial_status='HTTP initial', final_url='Finale URL', redirects='Redirects', response_ms='Antwortzeit', indexable='Indexierbarkeit', error='Abruffehler')
TITLE_KEYS = ['url', 'title_text', 'title_length', 'title_pixels', 'title_count', 'duplicates', 'status']
PROJECTED_KEYS = ['url', 'status', 'title_text', 'title_pixels', 'title_count', 'duplicates', 'title_length']
STATUS_KEYS = ['url', 'initial_status', 'status', 'final_url', 'redirects', 'response_ms', 'indexable', 'error']
PROFILE = dict(version=1, respect_noindex=True, respect_canonical=False, respect_next_prev=False)
PROVENANCE_LABELS = {prefix+'-'+suffix for prefix in ['warm-limited','cold-limited','completed-before-cold','cold-completed'] for suffix in ['ordinary','status','segment-reported','segment-raw','raw-status-bools','raw-text-filter-false','raw-text-filter-true','empty-segment']} | {'warm-pending-retained-only','limited-raw320','completed-raw320'}
AVAILABILITY_LABELS = {'warm-titles-under-analysis-fault','warm-loaded-empty-structured','warm-known-inputs','warm-known-input-empty-query','cold-titles-healthy','cold-loaded-empty-structured','cold-known-inputs','cold-known-input-empty-query'}


def digest(value): return hashlib.sha256(value).hexdigest()
def compact(value): return json.dumps(value, ensure_ascii=False, separators=(',', ':'))
def pathname(page): return urlsplit(page['url']).path


def top_pages_bytes(raw):
    """Read the native pretty JSON's top-level pages array without reserializing."""
    matches=list(re.finditer(rb'(?m)^  "pages": (\[)',raw));assert len(matches)==1
    start=matches[0].start(1);depth=0;quoted=False;escaped=False
    for index in range(start,len(raw)):
        byte=raw[index]
        if quoted:
            if escaped:escaped=False
            elif byte==92:escaped=True
            elif byte==34:quoted=False
        elif byte==34:quoted=True
        elif byte in [91,123]:depth+=1
        elif byte in [93,125]:
            depth-=1
            if depth==0:return raw[start:index+1]
    raise AssertionError('Unterminated original pages array')


def kind(value):
    if value is None: return 'null'
    if isinstance(value, bool): return 'boolean'
    if isinstance(value, str): return 'string'
    if isinstance(value, (int, float, decimal.Decimal)): return 'number'
    raise AssertionError(('Unsupported scalar', type(value)))


class Audit:
    def __init__(self, run):
        self.run = run
        self.pins = {}
        self.files = []
        self.controls = []
        self.receipts = []
        self.segment_tokens = set()
        self.source_bindings = []
        self.native_segment_reads_checked = 0
        self.seen_pairs = set()
        self.seen_paths = set()

    def read(self, path):
        path = path.resolve()
        assert path.is_relative_to(self.run), ('Source escaped explicit run root', path)
        assert path.is_file() and not path.is_symlink()
        data = path.read_bytes()
        name = str(path.relative_to(ROOT))
        assert self.pins.setdefault(name, digest(data)) == digest(data), ('Mutable input', name)
        return data

    def json(self, path): return json.loads(self.read(path).decode('utf-8-sig'))

    def original_capture(self, directory, record):
        label = record['label']
        raw = self.read(directory / (label + '-original-project.json'))
        report = json.loads(raw)
        assert digest(raw) == record['project_sha256'] and len(report['pages']) == record['pages']
        canonical = self.read(directory / (label + '-canonical-generation.json'))
        assert digest(canonical) == record['canonical_sha256'] and json.loads(canonical) == report
        return dict(record=record, report=report, canonical=canonical, original=raw)

    def profile(self, report):
        assert report['config']['directive_reporting'] == PROFILE
        hidden = []
        for page in report['pages']:
            noindex = any('noindex' in re.split(r'[\s,]+', value.lower()) for value in page['meta_robots'])
            assert noindex == (pathname(page) == '/products/hidden'), 'Declared fixture and captured native directive disagree'
            if noindex: hidden.append(page)
        assert len(hidden) == 1
        return [len(report['pages']), len(report['pages']) - len(hidden), len(hidden)]

    def rules(self, report):
        origin = '{0.scheme}://{0.netloc}'.format(urlsplit(report['pages'][0]['url']))
        def definition(identifier, name, field, operator, value):
            return dict(id=identifier, name=name, join='all', conditions=[dict(field=field, operator=operator, value=value, case_insensitive=False)])
        return dict(version=1, definitions=[
            definition('s1', 'Products and selected redirect sources', 'requested_url', 'regex', r'/(?:products/(?:[0-9]{4}|hidden)|redirect-[ab])(?:\?|$)'),
            definition('s2', 'Stored Product title overlap', 'stored_title', 'contains', 'Product'),
            definition('s3', 'Known empty uncaptured target', 'requested_url', 'equals', origin + '/never-captured')])

    def context(self, capture, observed, mode, identifier):
        record, report = capture['record'], capture['report']
        selection = dict(kind='segment', id=identifier)
        identity = dict(engine='open-seo-spider/saved-segments-reporting-v1', mode=mode, directive_reporting=PROFILE)
        # Origin is explicitly App-declared context, not independent checkpoint/SQL attestation.
        origin = observed['origin']
        assert origin in ['app_declared_job_report_with_cached_checkpoint', 'app_declared_job_report_without_cached_checkpoint']
        rules_hash = digest(compact(self.rules(report)).encode())
        token_tuple = ['saved_segments_v1', record['id'], record['generation'], origin, digest(capture['canonical']), rules_hash, selection, identity]
        wanted = dict(engine='saved_segments_v1', job_id=record['id'], app_generation=record['generation'], origin=origin,
                      typed_report_sha256=digest(capture['canonical']), rules_sha256=rules_hash, reporting_view=identity,
                      snapshot_token=digest(compact(token_tuple).encode()), sql_attested=False, report_generation_attested=False)
        assert observed == wanted, 'Native context differs from independent generation/rules/selection/hash reconstruction'
        self.segment_tokens.add(wanted['snapshot_token'])
        return wanted, selection

    def title_rows(self, report, selected):
        pages = [(ordinal, report['pages'][ordinal]) for ordinal in selected if report['pages'][ordinal]['internal'] and report['pages'][ordinal]['content_type'].split(';')[0] in ['text/html', 'application/xhtml+xml']]
        counts = collections.Counter(page['titles'][0].strip() for _, page in pages if page['titles'] and page['titles'][0].strip())
        result = []
        for ordinal, page in pages:
            title = page['titles'][0] if page['titles'] else None
            result.append(dict(url=page['url'], title_text=title, title_length=len(title or ''), title_pixels=(page['title_pixel_widths'] or [None])[0],
                               title_count=len(page['titles']), duplicates=counts.get((title or '').strip(), 0), status=page['final_status'] if page['final_status'] is not None else page['status'], page=page, ordinal=ordinal))
        return result

    def status_rows(self, report, selected):
        result = []
        for ordinal in selected:
            p = report['pages'][ordinal]
            if not p['internal']: continue
            assert p['indexable'] is None or isinstance(p['indexable'], bool)
            result.append(dict(url=p['url'], initial_status=p['status'], status=p['final_status'] if p['final_status'] is not None else p['status'], final_url=p['final_url'],
                               redirects=len(p['redirect_chain']), response_ms=p['response_ms'], indexable=p['indexable'], error=p['error'], page=p, ordinal=ordinal))
        return result

    def reconstruct_provenance(self, entry, captures):
        label = entry['label']
        if label == 'warm-pending-retained-only':
            parent, child = captures['parent'], captures['limited']
            targets = {target['requested_url'] for target in child['report']['operation']['targets']}
            assert len(targets) == 2
            report = dict(parent['report'])
            report['pages'] = [page for page in report['pages'] if page['url'] not in targets]
            report['operation'] = child['report']['operation']
            assert len(report['pages']) == 114
            capture = dict(report=report, record=child['record'])
            source = 'Genuine immutable parent minus both exact selected requested targets; stable child operation supplies invariant parent/hash/coverage. No persisted running full report is invented.'
        else:
            source_label = 'cold-completed' if label.startswith('cold-completed') else 'cold-limited' if label.startswith('cold-limited') else 'completed' if label.startswith('completed') else 'limited'
            capture = captures[source_label]
            report = capture['report']
            source = source_label + '-original-project.json + canonical-generation.json'
        counts = self.profile(report)
        context = entry.get('context')
        is_segment = context is not None
        mode = ('raw' if any(text in label for text in ['segment-raw', 'raw-status', 'raw-text', 'raw320']) else 'reported') if is_segment else 'reported'
        identifier = 's3' if 'empty-segment' in label else 's1'
        selected = []
        for ordinal, p in enumerate(report['pages']):
            if mode == 'reported' and pathname(p) == '/products/hidden': continue
            if is_segment and (identifier == 's3' or not re.fullmatch(r'/products/(?:\d{4}|hidden)|/redirect-[ab]', pathname(p))): continue
            selected.append(ordinal)
        wanted_context, selection = self.context(capture, context, mode, identifier) if is_segment else (None, None)
        section = entry['section']
        keys = STATUS_KEYS if section == 'status' else PROJECTED_KEYS
        rows = self.status_rows(report, selected) if section == 'status' else self.title_rows(report, selected)
        if 'raw-text-filter-false' in label: rows = [row for row in rows if row['indexable'] is False]
        if 'raw-text-filter-true' in label: rows = [row for row in rows if row['indexable'] is True]
        # Fixture URLs are lower-case ASCII with zero-padded paths; code-point
        # order independently equals the declared German numeric URL order here.
        rows.sort(key=lambda row: row['url'])
        header = [LABELS[key] for key in keys] + PROVENANCE + (SEGMENT if is_segment else []) + REPORTING
        matrix = []
        op = report.get('operation')
        for row in rows:
            target = next((target for target in (op or {}).get('targets', []) if target['requested_url'] == row['url']), None)
            selected_state = target['state'] if op and op['coverage'] == 'mixed_age_selected_recrawl' and target else ''
            assert selected_state != 'pending', 'An absent pending historical PageRecord must not be invented'
            basis = 'captured' if not op else 'refreshed_in_selected_job' if selected_state == 'committed' else 'inherited_from_parent'
            provenance = [(op or {}).get('coverage', 'crawl_snapshot'), basis, selected_state, op['parent']['snapshot_sha256'] if op else '', row['page']['fetched_at'], row['url']]
            segment = [wanted_context['snapshot_token'], wanted_context['rules_sha256'], wanted_context['typed_report_sha256'], wanted_context['app_generation'], compact(selection), compact(wanted_context['reporting_view']), row['ordinal']] if is_segment else []
            matrix.append([row[key] for key in keys] + provenance + segment + counts)
        assert len(matrix) == entry['rows']
        return header, matrix, source

    def provenance(self, directory, evidence):
        captures = {record['label']: self.original_capture(directory, record) for record in evidence['captures'] if 'label' in record}
        if 'limited' in captures:
            parent_bytes = self.read(directory / 'actual-child-immutable-parent.json')
            assert json.loads(parent_bytes) == captures['parent']['report']
            assert digest(parent_bytes) == captures['limited']['report']['operation']['parent']['snapshot_sha256']
            assert parent_bytes == captures['parent']['canonical']
        # Contexts are genuine independently repeated read-only native POST data,
        # not original intercepted response bodies; keep that qualification.
        native_contexts = []
        for read in evidence.get('segment_native_reads', []):
            assert read['source'].startswith('Exact independently repeated native read-only POST')
            capture = next(c for c in captures.values() if c['record']['id'] == read['value']['context']['job_id'] and c['record']['generation'] == read['value']['context']['app_generation'])
            report, value, request = capture['report'], read['value'], read['request']
            assert read['request']['rules'] == self.rules(report)
            assert read['value']['cohort']['rules'] == self.rules(report)
            chosen = request['selection'];assert chosen['kind']=='segment' and chosen['id'] in ['s1','s3']
            mode = request.get('reporting_mode','reported')
            self.context(capture,value['context'],mode,chosen['id']);native_contexts.append(value['context'])
            ordinals=[i for i,p in enumerate(report['pages']) if (mode=='raw' or pathname(p)!='/products/hidden') and chosen['id']=='s1' and re.fullmatch(r'/products/(?:\d{4}|hidden)|/redirect-[ab]',pathname(p))]
            selected_urls={report['pages'][i]['url'] for i in ordinals}
            issues=[issue for issue in report['issues'] if issue['url'] in selected_urls]
            counts=self.profile(report);summary=value['cohort']['summary'];reporting=value['cohort']['reporting']
            assert [reporting['captured_page_observations'],reporting['profile_visible_page_observations'],reporting['profile_excluded_page_observations']]==counts
            assert reporting['eligible_page_observations']==(counts[0] if mode=='raw' else counts[1])
            assert reporting['selected_page_observations']==len(ordinals) and summary['selected_pages']==len(ordinals) and summary['selected_issues']==len(issues)
            assert value['captured_crawl_summary']==report['summary']
            if read['path'].endswith('/dashboard'):
                offset=(request.get('cursor') or {}).get('offset',0);assert request['limit']==200
                if request['view']=='pages':
                    selected=ordinals[offset:offset+200]
                    assert value['total']==len(ordinals)
                    assert [row['page_ordinal'] for row in value['rows']]==selected
                    assert [row['page'] for row in value['rows']]==[report['pages'][i] for i in selected]
                else:
                    assert request['view']=='issues' and value['total']==len(issues) and value['rows']==issues[offset:offset+200]
                if value['next'] is not None:assert value['next']['offset']==offset+len(value['rows'])
            self.native_segment_reads_checked+=1
        for entry in evidence['files']:
            if entry.get('context') is not None:assert entry['context'] in native_contexts,'Workbook context lacks genuine native read binding'
            header, rows, source = self.reconstruct_provenance(entry, captures)
            self.file_pair(directory, entry, header, rows, source)

    def availability(self, directory, evidence):
        captures,original_bytes = {},{}
        for record in evidence['sources']:
            raw = self.read(directory / (record['label'] + '-original-project.json'))
            assert digest(raw) == record['project_sha256']
            report = json.loads(raw)
            assert len(report['pages']) == record['pages']
            captures[record['label']] = report
            original_bytes[record['label']]=raw
        if 'legacy-missing-inputs' in captures:
            fresh,ordinary=captures['fresh-auto-inputs'],captures['legacy-missing-inputs']
            assert fresh['config']['input_imports'],'Fresh current native capture actually records seed provenance'
            source=original_bytes['fresh-auto-inputs'];derived=self.read(directory/'legacy-missing-input-provenance-request.json')
            wanted=json.loads(source);del wanted['config']['input_imports'];assert json.loads(derived)==wanted
            prefix=0
            while prefix<min(len(source),len(derived)) and source[prefix]==derived[prefix]:prefix+=1
            suffix=0
            while suffix<min(len(source),len(derived))-prefix and source[len(source)-1-suffix]==derived[len(derived)-1-suffix]:suffix+=1
            assert source[:prefix]+source[len(source)-suffix:]==derived,'Legacy request must be one contiguous omission, not a source rewrite'
            assert len(source)>len(derived)
            restored=json.loads(derived);restored['config']['input_imports']=[]
            assert ordinary==restored,'Native legacy import may default only the deliberately omitted provenance'
            pages=top_pages_bytes(source)
            assert top_pages_bytes(derived)==pages and top_pages_bytes(original_bytes['legacy-missing-inputs'])==pages,'Original genuine raw PageRecords remain byte-exact through omission/native import'
            self.json(directory/'legacy-input-provenance-omission-proof.json')
            self.source_bindings.append(dict(kind='deliberate_legacy_missing_input_provenance',source_sha256=digest(source),derived_request_sha256=digest(derived),native_defaulted_output_sha256=digest(original_bytes['legacy-missing-inputs']),raw_pages_sha256=digest(pages),removed_bytes=len(source)-len(derived),qualification='Negative legacy metadata omission from actual fresh native capture, then real native import. Positive PageRecords are unchanged; fresh input capture was not absent.'))
            ordinary_source='legacy-missing-inputs-original-project.json native-defaulted omission case, byte-exact genuine fresh PageRecords'
        else:
            ordinary=captures['ordinary'];ordinary_source='ordinary-original-project.json actual native PageRecords'
        imported=captures['actual-list-import']
        assert ordinary['config']['input_imports'] == []
        assert all(page['structured_data'] == [] and page['invalid_structured_data'] == 0 for page in ordinary['pages']), 'Real native PageRecords confirm no JSON-LD observations'
        inventories = imported['config']['input_imports']
        assert len(inventories) == 1
        inventory = inventories[0]
        origin = '{0.scheme}://{0.netloc}'.format(urlsplit(imported['pages'][0]['url']))
        declared_rows = [origin + '/a#part', origin + '/a', origin + '/b']
        declared_bytes = ('\n'.join(declared_rows) + '\n').encode()
        assert inventory['engine'] == 'open-seo-spider/offline-list-v1/calamine-0.36.1' and inventory['website_requests'] == 0
        assert inventory['input_sha256'] == digest(declared_bytes) and inventory['input_bytes'] == len(declared_bytes) and len(inventory['rows']) == 3
        for index, (row, declared) in enumerate(zip(inventory['rows'], declared_rows), 1):
            normalized, fragment = urldefrag(declared)
            assert row['raw_value'] == declared and row['source_index'] == index and row['source_line'] == index
            assert row['normalized_url'] == normalized and row['fragment'] == (fragment or None)
        for entry in evidence['files']:
            section, label = entry['section'], entry['label']
            if section == 'titles':
                header = [LABELS[key] for key in TITLE_KEYS] + PROVENANCE
                rows = self.title_rows(ordinary, range(len(ordinary['pages'])))
                rows.sort(key=lambda row: row['url'])
                data = [[row[key] for key in TITLE_KEYS] + ['crawl_snapshot', 'captured', '', '', row['page']['fetched_at'], row['url']] for row in rows]
                source = ordinary_source
            elif section == 'structured':
                header = ['URL', 'JSON-LD-Blöcke', 'Ungültiges JSON', 'Beobachtete @type-Werte', 'Knoten mit @type', 'Schema-Validierung'] + PROVENANCE
                data = []
                source = ordinary_source+'; each native PageRecord carries structured_data[]/invalid_structured_data0; genuine loaded-empty analysis journey'
            else:
                assert section == 'inputs'
                header = ['Liste', 'Quellhash', 'Quellindex', 'Physische Zeile', 'Originalwert', 'Normalisierte URL', 'Fragment'] + PROVENANCE
                data = [] if 'input-empty' in label else [[1, inventory['input_sha256'], index, index, declared, urldefrag(declared)[0], urldefrag(declared)[1] or None, 'crawl_snapshot', 'unknown_or_mixed_row_identity', '', '', '', ''] for index, declared in enumerate(declared_rows, 1)]
                source = 'Actual native saved input inventory plus independently reconstructed original owned TXT bytes/rows; zero output only from declared unmatched query'
            assert len(data) == entry['rows']
            self.file_pair(directory, entry, header, data, source)

    def file_pair(self, directory, entry, header, rows, source):
        label = entry['label']
        family=directory.parent.name;key=(family,label)
        assert label in (PROVENANCE_LABELS if family=='table-xlsx-provenance' else AVAILABILITY_LABELS)
        assert key not in self.seen_pairs,('Duplicated original file receipt',key)
        self.seen_pairs.add(key)
        xlsx_path=(directory/(label+'.xlsx')).resolve();csv_path=(directory/(label+'.csv')).resolve()
        assert Path(entry['path']).resolve()==xlsx_path,'Original receipt points to a different XLSX path'
        for actual_path in [xlsx_path,csv_path]:
            identity=str(actual_path).casefold()
            assert identity not in self.seen_paths,('Duplicate actual original file path',actual_path)
            self.seen_paths.add(identity)
        raw = self.read(xlsx_path)
        actual_csv = self.read(csv_path)
        assert digest(raw) == entry['sha256'] and digest(actual_csv) == entry['csv_sha256']
        expected_csv = csv_bytes(header, rows)
        assert actual_csv == expected_csv, (label, 'Complete original CSV bytes differ')
        parsed_csv = list(csv.reader(io.StringIO(actual_csv.decode('utf-8-sig'), newline='')))
        assert len(parsed_csv) == len(rows) + 1 and all(len(row) == len(header) for row in parsed_csv)
        cells, types = check_workbook(raw, [header] + rows)
        # Existing author expectations are only a redundant cross-check AFTER
        # source reconstruction; never used as source truth or for membership.
        expected_file = directory / (label + ('.expected.json' if 'provenance' in directory.parent.name else '.independent-expected.json'))
        if expected_file.is_file():
            author_expected = self.json(expected_file)
            assert author_expected['header'] == header and author_expected['rows'] == rows
        self.files.append(dict(label=label, family=directory.parent.name, original_native_result=self.receipts[-1]['result'], rows=len(rows), columns=len(header), physical_cells=cells,
                               xlsx_sha256=digest(raw), csv_sha256=digest(actual_csv), scalar_types=dict(types), source=source))
        if not self.controls:
            first = zipfile.ZipFile(io.BytesIO(raw)).infolist()[0]
            name_length, extra_length = struct.unpack_from('<HH', raw, first.header_offset + 26)
            offset = first.header_offset + 30 + name_length + extra_length
            corrupt = bytearray(raw); corrupt[offset] ^= 1
            try:
                with zipfile.ZipFile(io.BytesIO(corrupt)) as package:
                    rejected = package.testzip() is not None
            except (zipfile.BadZipFile, ET.ParseError): rejected = True
            assert rejected
            self.controls.append('Declared own-memory CRC corruption rejected by standard Python ZIP reader; original file unchanged')


def col(index):
    out = ''
    while index:
        index, rem = divmod(index - 1, 26)
        out = chr(65 + rem) + out
    return out


def check_workbook(raw, matrix):
    assert raw[-22:-18] == b'PK\x05\x06' and struct.unpack_from('<H', raw, len(raw)-2)[0] == 0
    types = collections.Counter()
    with zipfile.ZipFile(io.BytesIO(raw)) as package:
        assert package.namelist() == PARTS and package.testzip() is None
        assert all(info.compress_type == zipfile.ZIP_STORED and not info.flag_bits & 1 for info in package.infolist())
        documents = {}
        for name in PARTS:
            xml = package.read(name)
            assert b'<!DOCTYPE' not in xml and b'<!ENTITY' not in xml
            documents[name] = ET.fromstring(xml)
        book = documents['xl/workbook.xml']
        assert [sheet.attrib['name'] for sheet in book.find(NS+'sheets')] == ['Ergebnisse']
        sheet = documents['xl/worksheets/sheet1.xml']
        assert not list(sheet.iter(NS+'f')) and not list(sheet.iter(NS+'hyperlink'))
        assert sheet.find(NS+'dimension').attrib == dict(ref='A1:'+col(len(matrix[0]))+str(len(matrix)))
        data = sheet.find(NS+'sheetData')
        assert len(data) == len(matrix)
        for row_number, (physical, expected) in enumerate(zip(data, matrix), 1):
            assert physical.tag == NS+'row' and physical.attrib == dict(r=str(row_number)) and len(physical) == len(expected)
            for column, (cell, wanted) in enumerate(zip(physical, expected), 1):
                address = col(column)+str(row_number)
                assert cell.tag == NS+'c' and cell.attrib.get('r') == address
                wire = cell.attrib.get('t')
                if wire is None:
                    assert cell.attrib == dict(r=address) and not len(cell) and cell.text is None
                    actual, cell_type = None, 'null'
                elif wire == 'inlineStr':
                    assert cell.attrib == dict(r=address,t='inlineStr') and len(cell) == 1
                    inline = cell[0]
                    assert inline.tag == NS+'is' and not inline.attrib and len(inline) == 1
                    text = inline[0]
                    assert text.tag == NS+'t' and not len(text) and text.attrib == {'{http://www.w3.org/XML/1998/namespace}space':'preserve'}
                    actual = re.sub(r'_x([0-9A-Fa-f]{4})_', lambda m: chr(int(m[1],16)), text.text or '')
                    cell_type = 'string'
                else:
                    assert wire in ['n','b'] and cell.attrib == dict(r=address,t=wire) and len(cell) == 1 and cell[0].tag == NS+'v' and not len(cell[0])
                    lexical = cell[0].text
                    if wire == 'b':
                        assert lexical in ['0','1']; actual, cell_type = lexical == '1', 'boolean'
                    else:
                        actual, cell_type = decimal.Decimal(lexical), 'number'
                        assert actual.is_finite()
                assert cell_type == kind(wanted), (address, cell_type, kind(wanted), actual, wanted)
                if cell_type == 'number': assert actual == decimal.Decimal(str(wanted)), (address, actual, wanted)
                else: assert actual == wanted, (address, actual, wanted)
                types[cell_type] += 1
    return sum(types.values()), types


def csv_bytes(header, rows):
    def cell(value, label=None):
        if label == 'Indexierbarkeit' and isinstance(value,bool): value = 'Indexierbar' if value else 'Nicht indexierbar'
        if value is None: value = ''
        text = str(value)
        if isinstance(value,str) and re.match(r'^[\s\ufeff]*[=+@-]',text): text = "'"+text
        return '"'+text.replace('"','""')+'"'
    return ('\ufeff'+','.join(cell(v) for v in header)+''.join('\r\n'+','.join(cell(value,header[i]) for i,value in enumerate(row)) for row in rows)).encode('utf-8')


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--run',required=True);parser.add_argument('--out',required=True)
    args=parser.parse_args();run=(ROOT/args.run).resolve();out=(ROOT/args.out).resolve()
    assert run.is_relative_to(ROOT/'artifacts') and run.is_dir()
    assert out.is_relative_to(ROOT/'artifacts') and not out.exists(), 'Fresh ignored destination only'
    out.mkdir()
    own_source=Path(__file__).read_bytes();(out/'audit.py').write_bytes(own_source)
    audit=Audit(run)
    result=dict(result='FAIL_AUDIT',run=str(run.relative_to(ROOT)),auditor_sha256=digest(own_source),scope='Offline original-file audit only; no product/test-oracle/compiler/writer imports or runtime activity.')
    try:
        outer=audit.json(run/'proof.json')
        result['original_native_binary_sha256']=outer.get('binary_sha256')
        result['original_outer_complete']=outer.get('complete',False)
        result['original_outer_failure']=outer.get('failure')
        for family,receipt_name,method in [('table-xlsx-provenance','ui-table-xlsx-provenance-live.json',audit.provenance),('table-xlsx-availability','ui-table-xlsx-availability-live.json',audit.availability)]:
            matches=list((run/family).glob('*/'+receipt_name))
            assert len(matches)<=1
            if not matches:
                audit.receipts.append(dict(family=family,result='NO_FINAL_RECEIPT',audited=False));continue
            evidence=audit.json(matches[0]);audit.receipts.append(dict(family=family,result=evidence['result'],failure=evidence.get('failure'),audited=True))
            if evidence['result']=='PASS':
                expected_labels=PROVENANCE_LABELS if family=='table-xlsx-provenance' else AVAILABILITY_LABELS
                assert len(evidence['files'])==len(expected_labels) and {entry['label'] for entry in evidence['files']}==expected_labels,'Every exact declared scenario must occur once'
            if evidence['files']:method(matches[0].parent,evidence)
            else:audit.receipts[-1]['audited_files']=0
        for relative,before in audit.pins.items(): assert digest((ROOT/relative).read_bytes())==before,('Original changed during audit',relative)
        complete=outer.get('complete',False) and len(audit.receipts)==2 and all(item['result']=='PASS' for item in audit.receipts)
        if complete:
            assert collections.Counter(item['family'] for item in audit.files)=={'table-xlsx-provenance':35,'table-xlsx-availability':8}
            result['result']='PASS_ALL_ORIGINAL_43_PAIRS'
        else: result['result']='PARTIAL_AUDIT_ORIGINAL_FAILED_OR_INCOMPLETE'
    except Exception as error:
        import traceback
        result['failure']=traceback.format_exc()
    result.update(original_inner_results=audit.receipts,audited_pairs=len(audit.files),unique_family_label_pairs=len(audit.seen_pairs),unique_actual_original_paths=len(audit.seen_paths),reconstructed_data_rows=sum(item['rows'] for item in audit.files),physical_cells=sum(item['physical_cells'] for item in audit.files),distinct_bound_segment_tokens=len(audit.segment_tokens),native_segment_reads_independently_checked=audit.native_segment_reads_checked,source_bindings=audit.source_bindings,files=audit.files,pinned_original_inputs=audit.pins,negative_controls=audit.controls,
                  boundaries=['Original FAIL/missing final receipt is not relabelled PASS even when its partial files decode correctly.','Expected matrices are only redundant cross-checks after separate reconstruction; membership/scalars/context/rules derive from genuine original reports, declared fixture predicates and native context receipts.','Segment API contexts are exact independently repeated read-only POST data, not original captured browser bodies. App-declared origin and false SQL/report-generation-attestation flags remain qualified.','Pending retained cells derive from immutable genuine parent minus exact selected requested targets, plus invariant stable-child parent/hash/coverage. No full running report or positive pending PageRecord is invented.','Archived limited generation and later completed generation are separate canonical native outputs; no mutation/ancestry attestation beyond original bytes and native operation fields.','This does not establish Excel/LibreOffice GUI roundtrip, every schema/operator/browser/renderer, specialist bulk formats or manufacturer parity.'])
    (out/'audit.json').write_text(json.dumps(result,indent=2,ensure_ascii=True)+'\n',encoding='utf-8')
    print(json.dumps({key:value for key,value in result.items() if key not in ['files','pinned_original_inputs','boundaries']},indent=2))
    return 1 if result['result']=='FAIL_AUDIT' else 0


if __name__=='__main__': sys.exit(main())
