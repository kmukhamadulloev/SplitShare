#!/usr/bin/env python3
"""Linux release smoke test: run a copied binary with isolated native config."""
import json
import os
from pathlib import Path
import re
import shutil
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
    with (root / 'server.log').open('w+') as log:
        process = subprocess.Popen([str(executable), '--no-tray'], cwd=root, env=env, stdout=log, stderr=log)
        try:
            for _ in range(100):
                assert process.poll() is None, 'Server exited before startup'
                try:
                    with urllib.request.urlopen('http://127.0.0.1:8080/', timeout=1) as response:
                        html = response.read().decode()
                    break
                except (urllib.error.URLError, ConnectionError):
                    time.sleep(0.05)
            else:
                raise AssertionError('Startup timed out')
            for asset in re.findall(r'(?:src|href)="(/[^" ]+)"', html):
                with urllib.request.urlopen('http://127.0.0.1:8080' + asset) as response:
                    assert response.status == 200 and response.read()
            for path, expected_status, expected_code in [('/api/v1/files', 503, 'SHARE_NOT_CONFIGURED'), ('/j/private-token', 503, 'SHARE_NOT_CONFIGURED')]:
                try:
                    urllib.request.urlopen('http://127.0.0.1:8080' + path)
                except urllib.error.HTTPError as error:
                    assert error.code == expected_status
                    assert json.load(error)['error']['code'] == expected_code
                else:
                    raise AssertionError('Unconfigured feature unexpectedly available')
            config = json.loads((root / 'data/SplitShare/config.json').read_text())
            assert config['version'] == 1
            process.terminate()
            assert process.wait(timeout=12) == 0
            log.seek(0)
            assert 'private-token' not in log.read()
            print('PASS: standalone assets, API/join isolation, config creation, SIGTERM shutdown, token log redaction')
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
