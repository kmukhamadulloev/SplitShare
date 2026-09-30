#!/usr/bin/env python3
"""Linux real-process bounded-memory regression for streamed uploads."""
import http.client
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

binary = Path(__file__).resolve().parents[1] / 'target/release/splitshare'
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    share = root / 'share'
    share.mkdir()
    process = subprocess.Popen([str(binary), '--no-tray', '--root', str(share), '--bind', '127.0.0.1:43124'], env=dict(os.environ, XDG_DATA_HOME=str(root / 'config')), stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    def rss():
        for line in Path(f'/proc/{process.pid}/status').read_text().splitlines():
            if line.startswith('VmRSS:'):
                return int(line.split()[1]) * 1024
        raise AssertionError('RSS unavailable')
    try:
        for _ in range(100):
            assert process.poll() is None
            connection = http.client.HTTPConnection('127.0.0.1', 43124, timeout=5)
            try:
                connection.request('GET','/api/v1/status')
                response = connection.getresponse()
                assert response.status == 200
                response.read()
                break
            except OSError:
                time.sleep(.05)
            finally:
                connection.close()
        else:
            raise AssertionError('Server startup timed out')
        peaks = []
        for size in [8 * 1024 * 1024, 256 * 1024 * 1024]:
            connection = http.client.HTTPConnection('127.0.0.1',43124,timeout=30)
            connection.putrequest('POST', f'/api/v1/uploads?path=/file-{size}')
            for key,value in {'Content-Type':'application/octet-stream','Content-Length':str(size),'X-SplitShare-Request':'1','X-Transfer-ID':uuid.uuid4().hex,'X-Transfer-Key':uuid.uuid4().hex}.items():
                connection.putheader(key,value)
            connection.endheaders()
            peak = rss()
            block = b'x' * 65536
            for offset in range(0,size,len(block)):
                connection.send(block)
                if offset % (1024 * 1024) == 0:
                    peak = max(peak,rss())
            response = connection.getresponse()
            assert response.status == 201
            transfer = json.loads(response.read())
            assert transfer['transferred_bytes'] == size and transfer['state'] == 'completed'
            assert (share / f'file-{size}').stat().st_size == size
            peaks.append(max(peak,rss()))
            connection.close()
        assert peaks[1] - peaks[0] < 32 * 1024 * 1024, f'RSS grew with upload size: {peaks}'
        print(f'PASS: 8 MiB / 256 MiB uploads; peak RSS {peaks[0]/1048576:.2f} / {peaks[1]/1048576:.2f} MiB')
    finally:
        process.terminate()
        process.wait(timeout=12)
