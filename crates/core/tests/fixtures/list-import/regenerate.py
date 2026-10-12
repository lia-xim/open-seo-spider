"""Independently authored minimal XLSX fixtures; no network or vendor assets."""
from pathlib import Path
from io import BytesIO
from zipfile import ZipFile, ZipInfo, ZIP_DEFLATED

ROOT = Path(__file__).parent
ROOT.mkdir(exist_ok=True)
MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"

def inline(cell, value):
    return f'<c r="{cell}" t="inlineStr"><is><t>{value}</t></is></c>'

rows = [
    '<row r="1">' + inline("A1", "Label") + inline("B1", "URL") + '</row>',
    '<row r="2"><c r="B2" t="s"><v>0</v></c></row>',
    '<row r="3">' + inline("B3", "https://example.test/same?b=2&amp;a=1#part") + '</row>',
    '<row r="4"/>',
    '<row r="5"><c r="B5" t="b"><v>1</v></c></row>',
    '<row r="6"><c r="B6" t="str"><f>HYPERLINK("https://never-fetch.test", "label")</f><v>https://example.test/cached</v></c></row>',
    '<row r="7"><c r="B7" t="str"><f t="shared" si="0" ref="B7:B8">A7</f><v>https://example.test/shared</v></c></row>',
    '<row r="8"><c r="B8" t="str"><f t="shared" si="0"/><v>https://example.test/derived</v></c></row>',
    '<row r="9"><c r="B9" t="e"><v>#N/A</v></c></row>',
    '<row r="10"><c r="B10" t="d"><v>2026-10-12T00:00:00Z</v></c></row>',
    '<row r="11">' + inline("B11", "open-me") + '</row>',
    '<row r="12"><c r="B12"><f>A12</f></c></row>',
    '<row r="13">' + inline("B13", "https://example.test/fragment#kept") + '</row>',
    '<row r="14"/>',
]

parts = {
    "[Content_Types].xml": '<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/></Types>',
    "_rels/.rels": f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{REL}/officeDocument" Target="xl/workbook.xml"/></Relationships>',
    "xl/workbook.xml": f'<workbook xmlns="{MAIN}" xmlns:r="{REL}"><sheets><sheet name="URLs" sheetId="1" r:id="rId1"/><sheet name="Other" sheetId="2" r:id="rId2"/><sheet name="Hidden" sheetId="3" state="hidden" r:id="rId3"/></sheets></workbook>',
    "xl/_rels/workbook.xml.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' + ''.join(f'<Relationship Id="rId{i}" Type="{REL}/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in range(1, 4)) + '</Relationships>',
    "xl/sharedStrings.xml": f'<sst xmlns="{MAIN}" count="1" uniqueCount="1"><si><r><t>https://example.test/</t></r><r><t>same?b=2&amp;a=1#part</t></r></si></sst>',
    "xl/worksheets/sheet1.xml": f'<worksheet xmlns="{MAIN}" xmlns:r="{REL}"><dimension ref="A1:XFD1048576"/><sheetData>' + ''.join(rows) + '</sheetData><hyperlinks><hyperlink ref="B11" r:id="link1"/></hyperlinks></worksheet>',
    "xl/worksheets/_rels/sheet1.xml.rels": f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="link1" Type="{REL}/hyperlink" Target="https://never-fetch.test/with-secret-label" TargetMode="External"/></Relationships>',
}
for i in (2, 3):
    parts[f"xl/worksheets/sheet{i}.xml"] = f'<worksheet xmlns="{MAIN}"><sheetData><row r="1">' + inline("A1", "URL") + '</row><row r="2">' + inline("A2", f"https://example.test/sheet{i}") + '</row></sheetData></worksheet>'

def write(name, content):
    with ZipFile(ROOT / name, "w", compression=ZIP_DEFLATED) as archive:
        for path, data in content.items():
            info = ZipInfo(path, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = ZIP_DEFLATED
            archive.writestr(info, data.encode("utf-8"))

write("ordered-formulas.xlsx", parts)
hostile = dict(parts)
hostile["xl/sharedStrings.xml"] = parts["xl/sharedStrings.xml"].replace('uniqueCount="1"', 'uniqueCount="18446744073709551615"')
write("huge-shared-count.xlsx", hostile)
hostile = dict(parts)
hostile["xl/worksheets/sheet1.xml"] = parts["xl/worksheets/sheet1.xml"].replace('<row r="14"/>', '<row r="1048576"/>')
write("huge-observed-row.xlsx", hostile)
hostile = dict(parts)
hostile["xl/worksheets/sheet1.xml"] = parts["xl/worksheets/sheet1.xml"].replace('<c r="B2" t="s"><v>0</v></c>', '<c r="B2" t="s"><v>0</v></c><c r="B2" t="s"><v>0</v></c>')
write("duplicate-cell.xlsx", hostile)
hostile = dict(parts)
hostile["xl/worksheets/sheet1.xml"] = parts["xl/worksheets/sheet1.xml"].replace('r="B3"', 'r="A2"')
write("wrong-cell-row.xlsx", hostile)
hostile = dict(parts)
hostile["xl/_rels/workbook.xml.rels"] = parts["xl/_rels/workbook.xml.rels"].replace('Target="worksheets/sheet1.xml"', 'Target="https://never-fetch.test/sheet.xml" TargetMode="External"')
write("external-worksheet.xlsx", hostile)
hostile = dict(parts)
hostile["xl/sharedStrings.xml"] = f'<sst xmlns="{MAIN}" count="1" uniqueCount="1"><si><t>' + ('Z' * 100_000) + '</t></si></sst>'
write("inflation-budget.xlsx", hostile)
hostile = dict(parts)
hostile["XL/WORKBOOK.XML"] = parts["xl/workbook.xml"]
write("case-colliding-entry.xlsx", hostile)
hostile = dict(parts)
hostile["xl/worksheets/sheet1.xml"] = parts["xl/worksheets/sheet1.xml"].replace('<t>Label</t>', '<t>URL</t>')
write("duplicate-header.xlsx", hostile)
print("Created nine independent XLSX fixtures")
