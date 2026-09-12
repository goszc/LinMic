#!/usr/bin/env python3
import pathlib,re,xml.etree.ElementTree as ET,json
root=pathlib.Path(__file__).resolve().parents[1];base=root/'android/app/src/main/res'
def read(p):return {e.attrib['name']:e.text or '' for e in ET.parse(p).getroot() if e.tag=='string'}
original=read(base/'values/strings.xml')
for locale in ['pt-rBR','es','ru','zh-rCN','ja']:
    values=read(base/f'values-{locale}/strings.xml');assert values.keys()==original.keys(),locale
    for key,text in values.items():
        assert text.strip(),(locale,key)
        formats=lambda s:sorted(re.findall(r'%\d+\$[\d.]*[dfsu]',s))
        assert formats(text)==formats(original[key]),(locale,key)
folder=root/'linux/gui/linmic-gui/locales';keys=json.loads((folder/'en.json').read_text()).keys()
for p in folder.glob('*.json'):assert json.loads(p.read_text()).keys()==keys,p
print('Six Android and desktop locales have matching keys and format placeholders.')
