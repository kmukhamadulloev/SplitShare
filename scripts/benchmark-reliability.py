#!/usr/bin/env python3
"""Linux real-process benchmark; bounded buffers for 1 GiB upload/download."""
import argparse
import http.client
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid
parser = argparse.ArgumentParser()
parser.add_argument('--large-mib',type=int,default=1024)
parser.add_argument('--files',type=int,default=10000)
args = parser.parse_args()
assert args.large_mib >= 8 and args.files > 0
binary = Path(__file__).resolve().parents[1] / 'target/release/splitshare'
with tempfile.TemporaryDirectory(prefix='splitshare-benchmark-') as directory:
    root = Path(directory); share = root/'share'; share.mkdir(); many = share/'many'; many.mkdir()
    for index in range(args.files): (many/f'file-{index:06}.txt').touch()
    with (root/'log').open('w+') as log:
        process = subprocess.Popen([str(binary),'--no-tray','--root',str(share),'--bind','127.0.0.1:43129'],env=dict(os.environ,XDG_DATA_HOME=str(root/'config'),RUST_LOG='splitshare=info'),stdout=log,stderr=log)
        def connection(): return http.client.HTTPConnection('127.0.0.1',43129,timeout=60)
        def rss():
            return next(int(line.split()[1])*1024 for line in Path(f'/proc/{process.pid}/status').read_text().splitlines() if line.startswith('VmRSS:'))
        try:
            for _ in range(100):
                assert process.poll() is None
                try:
                    client = connection(); client.request('GET','/api/v1/status'); response=client.getresponse(); response.read(); client.close(); break
                except OSError: time.sleep(.05)
            else: raise AssertionError('Startup timed out')
            results = []; block=b'z'*65536
            for mib in dict.fromkeys([8,10,args.large_mib]):
                size=mib*1048576; peak=rss(); start=time.monotonic(); client=connection()
                client.putrequest('POST',f'/api/v1/uploads?path=/file-{mib}.bin')
                for key,value in {'Content-Type':'application/octet-stream','Content-Length':str(size),'X-SplitShare-Request':'1','X-Transfer-ID':uuid.uuid4().hex,'X-Transfer-Key':uuid.uuid4().hex}.items(): client.putheader(key,value)
                client.endheaders()
                for offset in range(0,size,len(block)):
                    client.send(block)
                    if offset % 1048576 == 0: peak=max(peak,rss())
                response=client.getresponse(); assert response.status==201
                transfer=json.loads(response.read()); assert transfer['state']=='completed' and transfer['transferred_bytes']==size
                client.close(); upload_seconds=time.monotonic()-start
                client=connection(); start=time.monotonic(); client.request('GET',f'/api/v1/files/download?path=/file-{mib}.bin')
                response=client.getresponse(); assert response.status==200 and int(response.getheader('Content-Length'))==size
                count=0
                while chunk:=response.read(65536):
                    assert chunk==block[:len(chunk)]; count+=len(chunk)
                    if count % 1048576 == 0: peak=max(peak,rss())
                assert count==size; client.close()
                results.append({'mib':mib,'upload_seconds':round(upload_seconds,3),'download_seconds':round(time.monotonic()-start,3),'peak_rss_mib':round(peak/1048576,2)})
            assert results[-1]['peak_rss_mib']-results[0]['peak_rss_mib']<32,results
            small_start=time.monotonic(); client=connection()
            for index in range(100):
                data=b'x' * (1 if index == 0 else 1024)
                path=f'/small-{index}.txt'
                client.request('POST',f'/api/v1/uploads?path={path}',body=data,headers={'Content-Type':'application/octet-stream','X-SplitShare-Request':'1','X-Transfer-ID':uuid.uuid4().hex,'X-Transfer-Key':uuid.uuid4().hex})
                response=client.getresponse(); assert response.status==201
                assert json.loads(response.read())['state']=='completed'
                client.request('GET',f'/api/v1/files/download?path={path}')
                response=client.getresponse(); assert response.status==200 and response.read()==data
            client.close(); small_seconds=time.monotonic()-small_start
            start=time.monotonic(); client=connection(); client.request('GET','/api/v1/files?path=/many'); response=client.getresponse(); assert response.status==200
            payload=response.read(); entries=json.loads(payload)['entries']; client.close()
            assert len(entries)==args.files
            assert all(entry['path'].startswith('/many/') for entry in entries)
            print(json.dumps({'transfers':results,'small_files':{'count':100,'upload_and_download_seconds':round(small_seconds,3)},'directory':{'entries':args.files,'seconds':round(time.monotonic()-start,3),'response_bytes':len(payload),'rss_mib':round(rss()/1048576,2)}},indent=2))
        finally:
            process.terminate(); assert process.wait(timeout=12)==0
