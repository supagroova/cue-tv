#!/usr/bin/env python3
"""Exclude exact reviewed synthetic fixture lines from generic privacy rules only.

Gitleaks and forbidden-file checks are never filtered. Content, path and rule must
all match; changing a fixture requires review and an explicit new hash.
"""
import argparse, hashlib, json, sys
from pathlib import Path
p=argparse.ArgumentParser(); p.add_argument('--rule',required=True); p.add_argument('--history',action='store_true'); a=p.parse_args()
root=Path(__file__).resolve().parent.parent
try:
 entries=json.loads((root/'scripts/privacy-fixtures.json').read_text())
 allowed={(x['path'],x['sha256']) for x in entries if x['rule']==a.rule}
except (OSError,ValueError,KeyError,TypeError):
 # Fail closed: malformed or absent metadata grants no exceptions.
 allowed=set()
for raw in sys.stdin:
 line=raw.rstrip('\n')
 try:
  if a.history:
   prefix,body=line.split(': ',1); name=prefix.split(' ',1)[1]
  else:
   name,number=line.rsplit(':',1); name=name.removeprefix('./')
   target=(root/name).resolve()
   if not target.is_relative_to(root): raise ValueError('outside source')
   body=target.read_text().splitlines()[int(number)-1]
  digest=hashlib.sha256(body.encode()).hexdigest()
  if (name,digest) not in allowed: print(line)
 except (ValueError,IndexError,OSError,UnicodeError): print(line)
