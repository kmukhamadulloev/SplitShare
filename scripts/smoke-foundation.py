#!/usr/bin/env python3
"""Linux release smoke test: run a copied binary with isolated native config."""
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

binary = Path(__file__).resolve().parents[1] / 'target/release/splitshare'
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    executable = root / 'splitshare'
    shutil.copy2(binary, executable)
    env = dict(os.environ, XDG_DATA_HOME=str(root / 'data'))
    with socket.socket() as reservation:
        reservation.bind(('127.0.0.1', 0))
        port = reservation.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    with (root / 'server.log').open('w+') as log:
        process = subprocess.Popen([str(executable), '--no-tray', '--bind', f'127.0.0.1:{port}'], cwd=root, env=env, stdout=log, stderr=log)
        try:
            for _ in range(100):
                assert process.poll() is None, 'Server exited before startup'
                try:
                    with urllib.request.urlopen(base + '/', timeout=1) as response:
                        html = response.read().decode()
                    break
                except (urllib.error.URLError, ConnectionError):
                    time.sleep(0.05)
            else:
                raise AssertionError('Startup timed out')
            for asset in re.findall(r'(?:src|href)="(/[^" ]+)"', html):
                with urllib.request.urlopen(base + asset, timeout=3) as response:
                    assert response.status == 200 and response.read()
            with urllib.request.urlopen(base + '/api/v1/host/setup', timeout=3) as response:
                setup = json.load(response)
            assert setup['setup_required'] and not setup['folder_selected'], setup
            with urllib.request.urlopen(base + '/api/v1/status', timeout=3) as response:
                status = json.load(response)
            assert not status['sharing'], status
            for path, expected_status, expected_code in [
                ('/api/v1/files', 503, 'SETUP_REQUIRED'),
                ('/j/private-token', 503, 'SHARE_NOT_CONFIGURED'),
            ]:
                try:
                    urllib.request.urlopen(base + path, timeout=3)
                except urllib.error.HTTPError as error:
                    assert error.code == expected_status, (path, error.code, expected_status)
                    payload = json.load(error)
                    assert payload['error']['code'] == expected_code, (path, payload, expected_code)
                else:
                    raise AssertionError('Unconfigured feature unexpectedly available')
            config = json.loads((root / 'data/SplitShare/config.json').read_text())
            assert config['version'] == 1
            process.terminate()
            assert process.wait(timeout=12) == 0
            log.seek(0)
            assert 'private-token' not in log.read()
            print('PASS: standalone assets, mandatory setup, disabled sharing, API/join isolation, config creation, SIGTERM shutdown, token log redaction')
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
