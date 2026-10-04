#!/usr/bin/env python3
"""Build and verify a local Linux archive candidate (not a published release)."""
import hashlib
import json
import platform
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

repo = Path(__file__).resolve().parents[1]
if platform.system() != 'Linux' or platform.machine() != 'x86_64':
    raise SystemExit('This packaging checkpoint supports native Linux x86_64 only')
subprocess.run(['bash', 'scripts/build-release.sh'], cwd=repo, check=True)
metadata = json.loads(subprocess.check_output(
    ['cargo', 'metadata', '--locked', '--no-deps', '--format-version', '1'], cwd=repo))
version = next(p['version'] for p in metadata['packages'] if p['name'] == 'splitshare')
name = f'splitshare-{version}-linux-x86_64'
output = repo / 'release'
output.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(dir=output) as temporary:
    staging = Path(temporary)
    package = staging / name
    package.mkdir()
    shutil.copy2(Path(metadata['target_directory']) / 'release/splitshare', package / 'splitshare')
    shutil.copy2(repo / 'LICENSE', package / 'LICENSE')
    shutil.copy2(repo / 'docs/assets/logo.png', package / 'splitshare.png')
    shutil.copy2(repo / 'docs/LINUX_ARCHIVE.md', package / 'README.md')
    archive = staging / f'{name}.tar.gz'
    with tarfile.open(archive, 'w:gz') as bundle:
        bundle.add(package, arcname=name)
    subprocess.run(['python3', str(repo / 'scripts/smoke-linux-archive.py'), str(archive)], check=True)
    with archive.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    checksum = staging / f'{archive.name}.sha256'
    checksum.write_text(f'{digest}  {archive.name}\n')
    archive.replace(output / archive.name)
    checksum.replace(output / checksum.name)
print(f'Verified local archive candidate: {output / archive.name}')
