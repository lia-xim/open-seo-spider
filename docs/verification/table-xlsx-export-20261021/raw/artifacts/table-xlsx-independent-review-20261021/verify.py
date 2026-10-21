import hashlib
import json
import math
import re
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET
import openpyxl

base = Path(__file__).resolve().parent
expected = json.loads((base / 'expected.json').read_text(encoding='utf-8'))
matrix = [expected['header']] + expected['rows']
ns = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
with zipfile.ZipFile(base / 'independent-cases.xlsx') as package:
    assert package.testzip() is None
    assert package.namelist() == ['[Content_Types].xml', '_rels/.rels', 'xl/workbook.xml', 'xl/_rels/workbook.xml.rels', 'xl/worksheets/sheet1.xml']
    documents = {name: ET.fromstring(package.read(name)) for name in package.namelist()}
    sheet = documents['xl/worksheets/sheet1.xml']
    rows = sheet.find(ns + 'sheetData')
    actual, kinds = [], []
    for row_index, row in enumerate(rows, 1):
        assert row.tag == ns + 'row' and row.attrib == {'r': str(row_index)}
        values, types = [], []
        for col_index, cell in enumerate(row):
            assert cell.tag == ns + 'c'
            address = chr(ord('A') + col_index) + str(row_index)
            assert cell.attrib['r'] == address
            kind = cell.attrib.get('t')
            if kind is None:
                assert len(cell) == 0 and cell.text is None
                value, type_name = None, 'null'
            elif kind == 'inlineStr':
                text = cell.find(ns + 'is/' + ns + 't')
                assert text is not None and len(text) == 0
                assert text.attrib == {'{http://www.w3.org/XML/1998/namespace}space': 'preserve'}
                value = re.sub(r'_x([a-fA-F0-9]{4})_', lambda m: chr(int(m[1], 16)), text.text or '')
                type_name = 'string'
            elif kind == 'n':
                lexical = cell.findtext(ns + 'v')
                value = float(lexical)
                assert math.isfinite(value)
                type_name = 'number'
            elif kind == 'b':
                lexical = cell.findtext(ns + 'v')
                assert lexical in ['0', '1']
                value, type_name = lexical == '1', 'boolean'
            else:
                raise AssertionError('Unexpected type: ' + repr(kind))
            values.append(value)
            types.append(type_name)
        actual.append(values)
        kinds.append(types)
    assert actual == matrix
    assert kinds[1][0] == 'null' and kinds[2][0] == 'string'
    assert kinds[3][0] == 'number' and kinds[5][0] == 'boolean'
    assert sheet.find(ns + 'dimension').attrib == {'ref': 'A1:B' + str(len(matrix))}

# General third-party XLSX parser, separate from both Node implementations.
book = openpyxl.load_workbook(base / 'independent-cases.xlsx', data_only=False)
assert book.sheetnames == ['Ergebnisse']
loaded = list(book.active.rows)
assert len(loaded) == len(matrix) and all(len(row) == 2 for row in loaded)
assert loaded[1][0].value is None and loaded[1][0].data_type == 'n'
assert loaded[2][0].value == '' and loaded[2][0].data_type == 's'
assert loaded[3][0].value == 0 and loaded[3][0].data_type == 'n'
assert loaded[5][0].value is True and loaded[5][0].data_type == 'b'
assert loaded[7][0].value == '=SUM(A1:A2)' and loaded[7][0].data_type == 's'
assert not any(cell.data_type == 'f' for row in loaded for cell in row)
non_xstring_checks = 0
for got_row, expected_row in zip(loaded, matrix):
    for got, want in zip(got_row, expected_row):
        if isinstance(want, str) and (re.search(r'_x[0-9a-fA-F]{4}_', want) or '\r' in want or any(ord(c) < 32 and c not in '\n\t' or ord(c) in [0xfffe, 0xffff] for c in want)):
            continue
        assert got.value == want
        non_xstring_checks += 1
assert loaded[11][0].value != matrix[11][0], 'Reader does not decode Xstring-protected literal inline text'
print(json.dumps({'zipfile_crc': 'PASS', 'all_five_parts_standard_xml': 'PASS', 'independent_typed_matrix': 'PASS', 'physical_rows': len(matrix), 'physical_columns': 2, 'null_and_empty_are_distinct': True, 'openpyxl_version': openpyxl.__version__, 'openpyxl_standard_package_and_types': 'PASS', 'openpyxl_non_xstring_cells_compared': non_xstring_checks, 'openpyxl_xstring_scope': 'openpyxl 3.1.5 exposes protected inline Xstring text verbatim; it is not an Xstring-decoding oracle or Excel application proof'}, ensure_ascii=True))
