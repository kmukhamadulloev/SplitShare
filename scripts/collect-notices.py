#!/usr/bin/env python3
"""Collect dependency license files and metadata without copying source paths."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess

repo = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--target', required=True)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1',
                                               '--filter-platform', args.target], cwd=repo))
packages = {p['id']: p for p in metadata['packages']}
nodes = {n['id']: n for n in metadata['resolve']['nodes']}
root = next(p['id'] for p in metadata['packages'] if p['name'] == 'splitshare')
pending, visited = [root], set()
while pending:
    current = pending.pop()
    if current in visited:
        continue
    visited.add(current)
    pending.extend(d['pkg'] for d in nodes[current]['deps']
                   if any(k['kind'] != 'dev' for k in d['dep_kinds']))
args.output.mkdir(parents=True, exist_ok=True)
entries, missing = [], []


def collect(kind, name, version, directory, license_id, license_file=None):
    destination = args.output / kind / f'{name.replace("/", "_")}@{version}'
    destination.mkdir(parents=True)
    files = set()
    for candidate in directory.iterdir():
        if candidate.is_file() and candidate.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT')):
            files.add(candidate)
        if candidate.is_dir() and candidate.name.lower() in ('license', 'licenses'):
            files.update(p for p in candidate.rglob('*') if p.is_file())
    if license_file:
        candidate = directory / license_file
        if candidate.is_file():
            files.add(candidate)
    supplemental = repo / 'docs/third-party' / f'{name}@{version}'
    if supplemental.is_dir():
        files.update(p for p in supplemental.iterdir() if p.is_file())
    for index, source in enumerate(sorted(files)):
        shutil.copyfile(source, destination / f'{index}-{source.name}')
    entries.append({'ecosystem': kind, 'name': name, 'version': version, 'license': license_id})
    if not files:
        missing.append(f'{kind}: {name}@{version}')


for identifier in sorted(visited):
    p = packages[identifier]
    if identifier in metadata['workspace_members']:
        continue
    collect('rust', p['name'], p['version'], Path(p['manifest_path']).parent, p['license'], p.get('license_file'))
lock = json.loads((repo / 'web/package-lock.json').read_text(encoding='utf-8'))
for location, entry in sorted(lock['packages'].items()):
    if not location or (entry.get('dev') and not location.endswith('/tailwindcss')):
        continue
    directory = repo / 'web' / location
    package = json.loads((directory / 'package.json').read_text(encoding='utf-8'))
    collect('npm', package['name'], package['version'], directory, package.get('license'))
(args.output / 'inventory.json').write_text(json.dumps(entries, indent=2) + '\n', encoding='utf-8')
if missing:
    raise SystemExit('Missing dependency license files; review before packaging:\n' + '\n'.join(missing))
print(f'Collected license files for {len(entries)} dependencies')
