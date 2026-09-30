#!/usr/bin/env python3
"""Native Linux tray smoke. Requires a desktop session and /usr/bin/python3 with Gio.
Exercises only the spawned SplitShare process through its exported native menu.
"""
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.request
from gi.repository import Gio, GLib

binary = Path(__file__).resolve().parents[1] / 'target/debug/splitshare'
bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
http = urllib.request.build_opener(urllib.request.ProxyHandler({}))

def call(dest, path, interface, method, signature, args):
    return bus.call_sync(dest, path, interface, method, GLib.Variant(signature,args), None, Gio.DBusCallFlags.NONE, 3000, None).unpack()

def until(check):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        value = check()
        if value: return value
        time.sleep(.05)
    raise AssertionError('Native tray smoke timed out')

with tempfile.TemporaryDirectory(prefix='splitshare-native-tray-') as directory:
    root = Path(directory); share = root / 'share'; share.mkdir()
    with (root / 'log').open('w+') as log:
        process = subprocess.Popen([str(binary),'--root',str(share),'--bind','127.0.0.1:43126'],env=dict(os.environ,XDG_DATA_HOME=str(root/'config'),RUST_LOG='splitshare=info'),stdout=log,stderr=log)
        try:
            def service():
                assert process.poll() is None, 'Native process exited'
                items = call('org.kde.StatusNotifierWatcher','/StatusNotifierWatcher','org.freedesktop.DBus.Properties','Get','(ss)',('org.kde.StatusNotifierWatcher','RegisteredStatusNotifierItems'))[0]
                for item in items:
                    name, _, path = item.partition('/')
                    owner = call('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','GetConnectionUnixProcessID','(s)',(name,))[0]
                    if owner == process.pid: return name, '/' + path if path else '/StatusNotifierItem'
                return None
            destination, item_path = until(service)
            menu = call(destination,item_path,'org.freedesktop.DBus.Properties','Get','(ss)',('org.kde.StatusNotifierItem','Menu'))[0]
            def entries():
                layout = call(destination,menu,'com.canonical.dbusmenu','GetLayout','(iias)',(0,-1,[]))[1]
                return {child[1].get('label'): (child[0],child[1]) for child in layout[2]}
            def click(label):
                item = until(lambda: entries().get(label))
                assert item[1].get('enabled',True)
                call(destination,menu,'com.canonical.dbusmenu','Event','(isvu)',(item[0],'clicked',GLib.Variant('i',0),0))
            until(lambda: 'Sharing' in entries())
            expected = {'Open SplitShare','Open shared folder','Copy share link','Stop sharing','Settings','Quit'}
            assert expected <= entries().keys()
            with http.open('http://127.0.0.1:43126/api/v1/host/network') as response:
                old_link = json.load(response)['candidates'][0]['url']
            click('Stop sharing'); until(lambda: 'Stopped' in entries())
            with socket.socket() as probe: assert probe.connect_ex(('127.0.0.1',43126)) != 0
            assert not entries()['Copy share link'][1].get('enabled',True)
            click('Start sharing'); until(lambda: 'Sharing' in entries())
            with http.open('http://127.0.0.1:43126/api/v1/host/network') as response:
                assert json.load(response)['candidates'][0]['url'] != old_link
            # Leave a real upload body unfinished, then invoke the native Quit item.
            with socket.create_connection(('127.0.0.1',43126)) as upload:
                headers = ('POST /api/v1/uploads?path=/unfinished.bin HTTP/1.1\r\nHost: 127.0.0.1:43126\r\nX-SplitShare-Request: 1\r\nContent-Type: application/octet-stream\r\nTransfer-Encoding: chunked\r\nX-Transfer-Id: '+ '1'*32 +'\r\nX-Transfer-Key: '+ '2'*32 +'\r\n\r\n')
                upload.sendall(headers.encode()+b'10000\r\n'+b'x'*65536+b'\r\n')
                until(lambda: list(share.iterdir()))
                click('Quit')
                assert process.wait(timeout=12) == 0
            assert not list(share.iterdir())
            log.seek(0); output = log.read()
            assert 'Native tray ready' in output
            assert old_link not in output
            print('PASS: Linux native menu, Stop/Start on same port, fresh link, Quit cancels active upload and cleans partial file')
        except Exception:
            log.flush(); log.seek(0); print(log.read())
            raise
        finally:
            if process.poll() is None:
                process.terminate()
                try: process.wait(timeout=12)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
