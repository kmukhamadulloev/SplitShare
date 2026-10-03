#!/usr/bin/env python3
"""Real-process upload diagnostic fields and privacy at WARN and DEBUG levels."""
import http.client,os,pathlib,socket,subprocess,tempfile,time
for level in ['warn','splitshare=info,splitshare_application::transfers=debug']:
 with tempfile.TemporaryDirectory(prefix='splitshare-log-smoke-') as tmp:
  root=pathlib.Path(tmp);share=root/'share';share.mkdir()
  with (root/'log').open('w+') as log:
   p=subprocess.Popen([str(pathlib.Path(__file__).resolve().parents[1] / 'target/release/splitshare'),'--no-tray','--root',str(share),'--bind','127.0.0.1:43129'],env=dict(os.environ,XDG_DATA_HOME=str(root/'config'),NO_COLOR='1',RUST_LOG=level),stdout=log,stderr=log)
   try:
    for _ in range(100):
     assert p.poll() is None, "Test server exited before startup"
     try:
      c=http.client.HTTPConnection('127.0.0.1',43129,timeout=3);c.request('GET','/api/v1/status');r=c.getresponse();r.read();c.close()
      if r.status==200:break
     except OSError:time.sleep(.05)
    else: raise AssertionError("Test server startup timed out")
    assert p.poll() is None
    for index,total,path in [(1,3,'ok.bin'),(2,6,'partial.bin'),(3,3,'ok.bin')]:
     c=http.client.HTTPConnection('127.0.0.1',43129,timeout=5)
     c.putrequest('POST',f'/api/v1/uploads?path=/{path}')
     for k,v in {'Content-Type':'application/octet-stream','Content-Length':str(total),'X-SplitShare-Request':'1','X-Transfer-ID':f'{index:032x}','X-Transfer-Key':'b'*32}.items():c.putheader(k,v)
     c.endheaders();c.send(b'abc')
     if index==2:c.sock.shutdown(socket.SHUT_WR)
     try:r=c.getresponse();r.read()
     except http.client.HTTPException:pass
     c.close()
    time.sleep(.2)
   finally:p.terminate();p.wait(timeout=15)
   log.seek(0);lines=[l for l in log.read().splitlines() if 'Upload ' in l];combined='\n'.join(lines)
   for expected in ['UnexpectedEof','Storage(Conflict)','stage="receive"','stage="publish"','bytes_written=3','transfer_id='+f'{2:032x}','expected_bytes=Some(6)']:assert expected in combined,(expected,combined)
   if level!='warn':
    for expected in ['Upload accepted','Upload worker started','Upload completed','Upload publication started']:assert expected in combined,(expected,combined)
   for secret in [str(share),'b'*32,'ok.bin','partial.bin']:assert secret not in combined
   assert not (share/'partial.bin').exists()
   print(f'PASS: RUST_LOG={level}: release HTTP success, interrupted body, conflict, partial cleanup and log privacy')
   if level=='warn':print(next(l for l in lines if 'UnexpectedEof' in l))
