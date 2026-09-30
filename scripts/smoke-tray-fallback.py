#!/usr/bin/env python3
"""Linux release regression: display initialization failure leaves HTTP usable."""
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

binary = Path(__file__).resolve().parents[1] / 'target/release/splitshare'
http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
with tempfile.TemporaryDirectory(prefix='splitshare-tray-fallback-') as directory:
    root = Path(directory)
    with (root / 'log').open('w+') as log:
        env = dict(os.environ, XDG_DATA_HOME=str(root/'config'), DISPLAY=':65530', WAYLAND_DISPLAY='missing-splitshare-display', RUST_LOG='splitshare=info')
        process = subprocess.Popen([str(binary),'--bind','127.0.0.1:43128'],env=env,stdout=log,stderr=log)
        try:
            for _ in range(100):
                assert process.poll() is None
                try:
                    with http.open('http://127.0.0.1:43128/',timeout=1) as response:
                        assert response.status == 200
                    log.flush(); log.seek(0)
                    if 'Tray unavailable; server continues' in log.read(): break
                except urllib.error.URLError: pass
                time.sleep(.05)
            else: raise AssertionError('Tray failure did not recover to headless HTTP')
            process.terminate()
            assert process.wait(timeout=12) == 0
            print('PASS: failed native display initialization preserves HTTP and graceful signal shutdown')
        finally:
            if process.poll() is None: process.kill(); process.wait()
