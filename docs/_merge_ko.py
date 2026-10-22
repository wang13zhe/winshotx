# -*- coding: utf-8 -*-
import json, re, sys
sys.stdout.reconfigure(encoding='utf-8')

base = json.load(open('base-es-en.json', encoding='utf-8'))
parts = ['_ko_p1.json', '_ko_p2a.json', '_ko_p2b.json', '_ko_p3.json', '_ko_p4.json']
merged = {}
for p in parts:
    d = json.load(open(p, encoding='utf-8'))
    for k, v in d.items():
        if k in merged:
            print('DUP KEY:', k)
        merged[k] = v

missing = [k for k in base if k not in merged]
extra   = [k for k in merged if k not in base]
print('base:', len(base), 'merged:', len(merged))
print('missing:', len(missing))
for k in missing: print('   MISSING ->', repr(k)[:100])
print('extra:', len(extra))
for k in extra: print('   EXTRA ->', repr(k)[:100])
