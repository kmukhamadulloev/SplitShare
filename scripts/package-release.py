#!/usr/bin/env python3
"""Package a previously built native target, then smoke-test the extracted archive."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import sys
import tarfile
import tempfile

REPO = Path(__file__).resolve().parents[1]
TARGETS = {
    'x86_64-unknown-linux-gnu': ('Linux', 'x86_64', 'linux-x86_64'),
    'x86_64-pc-windows-msvc': ('Windows', 'AMD64', 'windows-x86_64'),
    'aarch64-apple-darwin': ('Darwin', 'arm64', 'macos-aarch64'),
    'x86_64-apple-darwin': ('Darwin', 'x86_64', 'macos-x86_64'),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', required=True, choices=TARGETS)
    args = parser.parse_args()
    system, machine, label = TARGETS[args.target]
    if platform.system() != system or platform.machine().lower() != machine.lower():
        raise SystemExit('Build and smoke-test on the native target runner')
    spec = importlib.util.spec_from_file_location('notes', REPO / 'scripts/release-notes.py')
    notes = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(notes)
    version = notes.release_version(REPO)
    release_notes = notes.extract_notes((REPO / 'RELEASE.md').read_text(encoding='utf-8'), version)
    metadata = json.loads(subprocess.check_output([
        'cargo', 'metadata', '--locked', '--format-version', '1', '--filter-platform', args.target], cwd=REPO))
    output = Path(metadata['target_directory']) / 'release-artifacts'
    output.mkdir(parents=True, exist_ok=True)
    binary = Path(metadata['target_directory']) / args.target / 'release' / ('splitshare.exe' if system == 'Windows' else 'splitshare')
    if not binary.is_file():
        raise SystemExit(f'Missing release executable: {binary}')
    name = f'splitshare-v{version}-{label}'
    with tempfile.TemporaryDirectory(dir=output) as temporary:
        stage = Path(temporary)
        package = stage / name
        package.mkdir()
        if system == 'Darwin':
            contents = package / 'SplitShare.app/Contents'
            (contents / 'MacOS').mkdir(parents=True)
            (contents / 'Resources').mkdir()
            shutil.copy2(binary, contents / 'MacOS/splitshare')
            (contents / 'MacOS/splitshare').chmod(0o755)
            info = {'CFBundleExecutable': 'splitshare', 'CFBundleIdentifier': 'com.splitshare.desktop',
                    'CFBundleName': 'SplitShare', 'CFBundleDisplayName': 'SplitShare',
                    'CFBundlePackageType': 'APPL', 'CFBundleVersion': version,
                    'CFBundleShortVersionString': version, 'CFBundleIconFile': 'SplitShare.icns',
                    'LSUIElement': True, 'NSHighResolutionCapable': True}
            with (contents / 'Info.plist').open('wb') as stream:
                plistlib.dump(info, stream)
            iconset = stage / 'SplitShare.iconset'
            iconset.mkdir()
            for size in [16, 32, 128, 256, 512]:
                for scale in [1, 2]:
                    suffix = '@2x' if scale == 2 else ''
                    subprocess.run(['sips', '-z', str(size * scale), str(size * scale),
                                    str(REPO / 'docs/assets/logo.png'), '--out',
                                    str(iconset / f'icon_{size}x{size}{suffix}.png')], check=True, stdout=subprocess.DEVNULL)
            subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o',
                            str(contents / 'Resources/SplitShare.icns')], check=True)
            subprocess.run(['plutil', '-lint', str(contents / 'Info.plist')], check=True)
            # Ad-hoc signing preserves executable integrity; it is not notarization.
            subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(contents.parent)], check=True)
        else:
            shutil.copy2(binary, package / binary.name)
        for source, destination in [('LICENSE', 'LICENSE'), ('docs/RELEASE_ARCHIVES.md', 'README.md'),
                                    ('docs/assets/logo.png', 'splitshare.png'),
                                    ('THIRD_PARTY_NOTICES.md', 'THIRD_PARTY_NOTICES.md')]:
            shutil.copy2(REPO / source, package / destination)
        (package / 'RELEASE.md').write_text(f'# SplitShare {version}\n\n{release_notes}', encoding='utf-8')
        subprocess.run([sys.executable, str(REPO / 'scripts/collect-notices.py'),
                        '--target', args.target, '--output', str(package / 'licenses')], check=True)
        if system == 'Windows':
            archive = Path(shutil.make_archive(str(stage / name), 'zip', stage, name))
        else:
            archive = stage / f'{name}.tar.gz'
            with tarfile.open(archive, 'w:gz') as bundle:
                bundle.add(package, arcname=name)
        subprocess.run([sys.executable, str(REPO / 'scripts/smoke-release-archive.py'), str(archive)], check=True)
        with archive.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        checksum = stage / f'{archive.name}.sha256'
        checksum.write_text(f'{digest}  {archive.name}\n', encoding='utf-8')
        archive.replace(output / archive.name)
        checksum.replace(output / checksum.name)
        print(output / archive.name)


if __name__ == '__main__':
    main()
