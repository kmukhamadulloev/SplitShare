#!/usr/bin/env python3
"""Validate release identity and extract only the requested changelog entry."""
import argparse
import json
from pathlib import Path
import re
import tomllib

REPO = Path(__file__).resolve().parents[1]


def extract_notes(content, version):
    sections = re.split(r'^##\s+', content, flags=re.MULTILINE)[1:]
    matches = [section.split('\n', 1)[1].strip() for section in sections
               if section.split('\n', 1)[0].split()[0] == version and '\n' in section]
    if len(matches) != 1 or not matches[0] or not re.search(r'^-\s+\S', matches[0], re.MULTILINE):
        raise ValueError(f'Expected one nonempty changelog entry for {version}')
    return matches[0] + '\n'


def release_version(repo, tag=None):
    version = tomllib.loads((repo / 'Cargo.toml').read_text(encoding='utf-8'))['workspace']['package']['version']
    if not re.fullmatch(r'\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?', version):
        raise ValueError('Invalid package version')
    if tag is not None and tag != f'v{version}':
        raise ValueError(f'Tag must match package version v{version}')
    package = json.loads((repo / 'web/package.json').read_text(encoding='utf-8'))
    lock = json.loads((repo / 'web/package-lock.json').read_text(encoding='utf-8'))
    project = json.loads((repo / 'project.json').read_text(encoding='utf-8'))
    if not all(v == version for v in [package['version'], lock['version'],
                                      lock['packages']['']['version'], project['version']]):
        raise ValueError('Frontend/project version mismatch')
    for package in tomllib.loads((repo / 'Cargo.lock').read_text(encoding='utf-8'))['package']:
        if package['name'] == 'splitshare' or package['name'].startswith('splitshare-'):
            if package['version'] != version:
                raise ValueError('Rust lockfile version mismatch')
    return version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    version = release_version(REPO, args.tag)
    notes = extract_notes((REPO / 'RELEASE.md').read_text(encoding='utf-8'), version)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(notes, encoding='utf-8')
    else:
        print(notes, end='')


if __name__ == '__main__':
    main()
