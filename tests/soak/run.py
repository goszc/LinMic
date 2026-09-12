#!/usr/bin/env python3
"""Bounded local soak; no root, no netem, no recording. Defaults to one hour."""
import argparse, json, os, pathlib, signal, socket, subprocess, tempfile, time
ROOT=pathlib.Path(__file__).resolve().parents[2]
def free_port():
    with socket.socket() as s:s.bind(('127.0.0.1',0));return s.getsockname()[1]
def main():
    p=argparse.ArgumentParser();p.add_argument('--seconds',type=int,default=3600);p.add_argument('--build',default='release');a=p.parse_args()
    assert a.seconds>=10
    binaries=ROOT/'target'/a.build
    with tempfile.TemporaryDirectory(prefix='linmic-soak-') as tmp:
        root=pathlib.Path(tmp);env=dict(os.environ);env['PIPEWIRE_RUNTIME_DIR']=os.environ['XDG_RUNTIME_DIR']
        for key,d in [('XDG_RUNTIME_DIR','run'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data')]:
            (root/d).mkdir(mode=0o700);env[key]=str(root/d)
        control,audio=free_port(),free_port();(root/'config/linmic').mkdir()
        (root/'config/linmic/config.toml').write_text(f'[network]\nbind="127.0.0.1"\ncontrol_port={control}\naudio_port={audio}\n[pipewire]\nnode_name="linmic_soak_{os.getpid()}"\nnode_description="LinMic soak test"\n[discovery]\nenabled=false\n')
        log=(root/'daemon.log').open('w+');daemon=subprocess.Popen([str(binaries/'linmicd')],env=env,stdout=log,stderr=log);sender=None
        samples=[];start=time.monotonic();hz=os.sysconf('SC_CLK_TCK');pagesize=os.sysconf('SC_PAGE_SIZE')
        def status():return json.loads(subprocess.check_output([str(binaries/'linmic'),'status','--json'],env=env,stderr=subprocess.DEVNULL))
        try:
            while True:
                try:
                    if status()['pipewire_connected']:break
                except subprocess.SubprocessError:pass
                if time.monotonic()-start>10:raise RuntimeError('daemon startup failed')
                time.sleep(.1)
            sender=subprocess.Popen([str(binaries/'linmic-test-sender'),'--host',f'127.0.0.1:{control}','--seconds',str(a.seconds+10)],env=env,stdout=log,stderr=log)
            start=time.monotonic()
            while time.monotonic()-start<a.seconds:
                time.sleep(1);s=status();proc=pathlib.Path(f'/proc/{daemon.pid}');fields=(proc/'stat').read_text().split();rss=int((proc/'statm').read_text().split()[1])*pagesize
                samples.append({'elapsed':time.monotonic()-start,'rss_bytes':rss,'cpu_seconds':(int(fields[13])+int(fields[14]))/hz,**s})
                assert daemon.poll() is None and sender.poll() is None
                assert s['state'] in ('STREAMING','MUTED'),s
                assert s['output_ring_ms']<=100 and s['decoder_errors']==0,s
            assert len({s['node_id'] for s in samples})==1
            warm=samples[min(10,len(samples)-1)];last=samples[-1]
            growth=last['rss_bytes']-warm['rss_bytes'];cpu=100*(last['cpu_seconds']-warm['cpu_seconds'])/max(1,last['elapsed']-warm['elapsed'])
            assert growth<5*1024*1024, growth
            assert last['estimated_latency_ms']<=samples[0]['estimated_latency_ms']+40
            result={'passed':True,'seconds':a.seconds,'rss_growth_bytes_after_warmup':growth,'rss_final_bytes':last['rss_bytes'],'average_cpu_percent_one_core':cpu,'last_stats':last,'samples':samples}
            (ROOT/'artifacts').mkdir(exist_ok=True);dest=ROOT/f'artifacts/soak-{a.seconds}s.json';dest.write_text(json.dumps(result,indent=2)+'\n')
            print(json.dumps({k:v for k,v in result.items() if k not in ('samples','last_stats')},indent=2),flush=True)
        finally:
            if sender and sender.poll() is None:sender.terminate();sender.wait(timeout=5)
            daemon.send_signal(signal.SIGINT)
            try:daemon.wait(timeout=8)
            except subprocess.TimeoutExpired:daemon.kill();daemon.wait()
            log.flush()
            if daemon.returncode not in (0,None):print((root/'daemon.log').read_text())
            log.close()
if __name__=='__main__':main()
