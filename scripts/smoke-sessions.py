#!/usr/bin/env python3
"""Linux standalone settings persistence, cookie/join and restart invalidation smoke."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.request
import urllib.error

binary = Path('target/release/splitshare').resolve()
base = 'http://127.0.0.1:43125'
class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None
opener = urllib.request.build_opener(NoRedirect)
def call(path, method='GET', body=None):
    headers = {'X-SplitShare-Request': '1'}
    data = None
    if body is not None:
        headers['Content-Type'] = 'application/json'
        data = json.dumps(body).encode()
    try:
        response = opener.open(urllib.request.Request(base + path, data=data, headers=headers, method=method), timeout=3)
    except urllib.error.HTTPError as response_error:
        response = response_error
    with response:
        payload = response.read()
        return response.status, response.headers, json.loads(payload) if payload else None
with tempfile.TemporaryDirectory(prefix='splitshare-sessions-') as directory:
    root = Path(directory)
    share = root / 'share'
    share.mkdir()
    env = dict(os.environ, XDG_DATA_HOME=str(root / 'config'))
    tokens = []
    with (root / 'server.log').open('w+') as log:
        for run in range(2):
            process = subprocess.Popen([str(binary), '--root', str(share), '--bind', '127.0.0.1:43125'], env=env, stdout=log, stderr=log)
            try:
                for _ in range(100):
                    assert process.poll() is None
                    try:
                        if call('/api/v1/status')[0] == 200:
                            break
                    except (urllib.error.URLError, ConnectionError):
                        time.sleep(0.05)
                else:
                    raise AssertionError('Startup timed out')
                settings = call('/api/v1/host/settings')[2]
                network = call('/api/v1/host/network')[2]
                link = network['candidates'][0]['url']
                token = link.rsplit('/',1)[1]
                tokens.append(token)
                if run == 0:
                    settings['permissions']['upload'] = False
                    settings['parallel_uploads_enabled'] = True
                    settings['max_parallel_uploads'] = 2
                    assert call('/api/v1/host/settings','PUT',settings)[0] == 200
                    code, headers, _ = call('/j/' + token)
                    assert code == 303 and 'HttpOnly' in headers['Set-Cookie']
                    assert headers['Location'] == '/'
                else:
                    assert not settings['permissions']['upload']
                    assert call('/api/v1/status')[2]['upload_concurrency'] == 2
                    assert tokens[0] != tokens[1]
                    assert call('/j/' + tokens[0])[0] == 401
                process.terminate()
                assert process.wait(timeout=12) == 0
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
        log.seek(0)
        logs = log.read()
        config = (root / 'config/SplitShare/config.json').read_text()
        assert all(token not in logs and token not in config for token in tokens)
print('PASS: settings survive restart, token rotates, cookie redirects, secrets absent from logs/config')
