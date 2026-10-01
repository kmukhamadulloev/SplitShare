#!/usr/bin/env python3
"""Linux/X11 native folder picker smoke; requires xdotool and a desktop portal."""
import http.client
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

xdotool = os.environ.get('SPLITSHARE_XDOTOOL', 'xdotool')
title = '^Choose the folder to share with SplitShare$'
def windows():
    result = subprocess.run([xdotool, 'search', '--onlyvisible', '--name', title], capture_output=True, text=True)
    assert result.returncode in (0, 1)
    return result.stdout.split()
def wait(check, seconds=15):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        result = check()
        if result: return result
        time.sleep(.1)
    raise AssertionError('Timed out waiting for host setup')
def api(path='/host/setup', method='GET'):
    client = http.client.HTTPConnection('127.0.0.1',43131,timeout=2)
    try:
        client.request(method, '/api/v1'+path, headers={'X-SplitShare-Request':'1'})
        response=client.getresponse(); body=response.read()
        assert response.status in (200,202), response.status
        return json.loads(body)
    finally: client.close()
def started():
    try: return api()
    except OSError: return None

def key(window, sequence):
    assert window in windows(), 'Refusing to type into an unrelated window'
    subprocess.run([xdotool,'windowactivate','--sync',window],check=True)
    assert subprocess.check_output([xdotool,'getactivewindow'],text=True).strip()==window
    subprocess.run([xdotool,'key','--clearmodifiers',sequence],check=True)

assert not windows(), 'Close existing SplitShare folder dialogs before running this smoke'
with tempfile.TemporaryDirectory(prefix='splitshare-picker-') as directory:
    root=Path(directory); share=root/'share'; share.mkdir(); (share/'selected.txt').write_text('native selection')
    binary=Path(__file__).resolve().parents[1]/'target/debug/splitshare'
    with (root/'host.log').open('w') as log:
        host=subprocess.Popen([str(binary),'--no-tray','--bind','127.0.0.1:43131'],env=dict(os.environ,XDG_DATA_HOME=str(root/'config')),stdout=log,stderr=log)
        owned=set()
        try:
            wait(started)
            assert not api()['folder_selected']
            assert api('/host/folder','POST')['state']=='selecting'
            window=wait(windows)[0]; owned.add(window)
            key(window,'Escape')
            wait(lambda: api()['state']=='cancelled')
            wait(lambda: not windows())
            assert not api()['folder_selected']
            assert api('/host/folder','POST')['state']=='selecting'
            window=wait(windows)[0]; owned.add(window)
            key(window,'ctrl+l')
            time.sleep(.2)
            assert window in windows()
            assert subprocess.check_output([xdotool,'getactivewindow'],text=True).strip()==window
            subprocess.run([xdotool,'type','--clearmodifiers','--delay','1','--',str(share)],check=True)
            if window in windows(): key(window,'Return')
            time.sleep(.5)
            if window in windows(): key(window,'alt+o')
            def ready():
                value=started()
                return value and value['state']=='ready' and value['folder_selected']
            wait(ready)
            assert any(entry['name']=='selected.txt' for entry in api('/files?path=/')['entries'])
            saved=json.loads((root/'config/SplitShare/host.json').read_text())
            assert saved['root']==str(share)
            print('PASS: real native folder dialog, cancellation, selection, live sandbox activation and private persistence')
        finally:
            host.terminate(); host.wait(timeout=12)
            # The portal owns the native window; close only our known dialog if still present.
            for window in owned.intersection(windows()):
                try: key(window,'Escape')
                except (AssertionError, subprocess.CalledProcessError):
                    if window in windows(): raise
