#!/usr/bin/env python3
"""Exercise an extracted archive with an empty PATH on a disposable native host."""
import http.client
import json
import os
from pathlib import Path
import re
import signal
import socket
import subprocess
import sys
import tarfile
import tempfile
import time
import zipfile

with tempfile.TemporaryDirectory() as temporary:
    root = Path(temporary)
    if sys.platform != 'linux' and os.environ.get('GITHUB_ACTIONS') != 'true':
        raise SystemExit('Windows/macOS smoke requires a disposable GitHub runner account')
    if sys.argv[1].endswith('.zip'):
        with zipfile.ZipFile(sys.argv[1]) as archive:
            archive.extractall(root)
    else:
        with tarfile.open(sys.argv[1]) as archive:
            archive.extractall(root, filter='data')
    binaries = (list(root.glob('*/splitshare')) + list(root.glob('*/splitshare.exe'))
                + list(root.glob('*/SplitShare.app/Contents/MacOS/splitshare')))
    assert len(binaries) == 1, 'Expected one packaged executable'
    share = root / 'share'
    share.mkdir()
    with socket.socket() as reservation:
        reservation.bind(('127.0.0.1', 0))
        port = reservation.getsockname()[1]
    env = dict(os.environ, PATH='', XDG_DATA_HOME=str(root / 'data'),
               XDG_CONFIG_HOME=str(root / 'config'))
    def request(path, method='GET', body=None, headers=None):
        client = http.client.HTTPConnection('127.0.0.1', port, timeout=3)
        try:
            client.request(method, path, body, headers or {})
            response = client.getresponse()
            return response.status, response.read(), dict(response.getheaders())
        finally:
            client.close()
    with (root / 'host.log').open('w+') as log:
        process = subprocess.Popen([str(binaries[0]), '--no-tray', '--root', str(share),
                                    '--bind', f'127.0.0.1:{port}'], cwd=root, env=env,
                                   stdout=log, stderr=log,
                                   creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == 'nt' else 0)
        try:
            for _ in range(100):
                assert process.poll() is None, 'Packaged host exited before startup'
                try:
                    status, html, _ = request('/')
                    break
                except OSError:
                    time.sleep(0.05)
            else:
                raise AssertionError('Packaged host startup timed out')
            assert status == 200
            assets = re.findall(r'(?:src|href)="(/[^" ]+)"', html.decode())
            assert any('/assets/' in asset for asset in assets)
            for asset in assets:
                status, data, _ = request(asset)
                assert status == 200 and data, asset
            status, data, _ = request('/api/v1/status')
            assert status == 200 and json.loads(data)['sharing']
            payload = b'SplitShare archive roundtrip\n' * 4096
            status, data, _ = request('/api/v1/uploads?path=/archive.bin', 'POST', payload, {
                'X-SplitShare-Request': '1', 'Content-Type': 'application/octet-stream',
                'X-Transfer-Id': '1' * 32, 'X-Transfer-Key': '2' * 32})
            assert 200 <= status < 300, (status, data)
            assert (share / 'archive.bin').read_bytes() == payload
            status, data, _ = request('/api/v1/files/download?path=/archive.bin')
            assert status == 200 and data == payload
            status, data, headers = request('/api/v1/files/download?path=/archive.bin',
                                            headers={'Range': 'bytes=3-17'})
            assert status == 206 and data == payload[3:18]
            if os.name == 'nt':
                # terminate() uses TerminateProcess on Windows, not SIGTERM.
                # Native tray/console graceful quit remains separate coverage.
                process.terminate()
                process.wait(timeout=12)
            else:
                process.send_signal(signal.SIGTERM)
                assert process.wait(timeout=12) == 0
            print('PASS: extracted archive, empty PATH, embedded assets, upload/download, Range, process cleanup')
        except BaseException:
            log.flush()
            log.seek(0)
            print(log.read(), file=sys.stderr)
            raise
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
