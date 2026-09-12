#!/usr/bin/env python3
"""Real TLS/Opus/UDP/PipeWire tests. Isolated config, keys, ports and virtual source."""
import json, math, os, pathlib, signal, socket, struct, subprocess, tempfile, time, wave
ROOT = pathlib.Path(__file__).resolve().parents[2]
BIN = ROOT / 'target' / os.environ.get('LINMIC_BUILD', 'debug')

def port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]

def main():
    with tempfile.TemporaryDirectory(prefix='linmic-integration-') as tmp:
        temp = pathlib.Path(tmp)
        env = dict(os.environ)
        for variable, directory in [('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_RUNTIME_DIR','run')]:
            env[variable] = str(temp/directory)
            (temp/directory).mkdir(mode=0o700)
        env['PIPEWIRE_RUNTIME_DIR'] = os.environ['XDG_RUNTIME_DIR']
        control, audio = port(), port()
        config = temp/'config/linmic'; config.mkdir()
        node = 'linmic_test_' + str(os.getpid())
        (config/'config.toml').write_text(f'[network]\nbind="127.0.0.1"\ncontrol_port={control}\naudio_port={audio}\n[pipewire]\nnode_name="{node}"\nnode_description="LinMic integration test"\n[discovery]\nenabled=false\n')
        log = (temp/'daemon.log').open('w+')
        daemon = subprocess.Popen([str(BIN/'linmicd')], env=env, stdout=log, stderr=log)
        senders = []
        def cli(*args):
            return json.loads(subprocess.check_output([str(BIN/'linmic'),*args],env=env,stderr=subprocess.PIPE))
        def wait(predicate, timeout=5):
            until=time.monotonic()+timeout
            while time.monotonic()<until:
                try:
                    value=cli('status','--json')
                    if predicate(value): return value
                except (subprocess.SubprocessError, json.JSONDecodeError): pass
                time.sleep(.05)
            log.flush(); print((temp/'daemon.log').read_text())
            raise AssertionError('state did not converge')
        def sender(*args):
            p=subprocess.Popen([str(BIN/'linmic-test-sender'),'--host',f'127.0.0.1:{control}','--seconds','30',*args],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            senders.append(p);return p
        def record():
            path=temp/'capture.wav'
            p=subprocess.Popen(['pw-record','--target',node,'--rate','48000','--channels','1',str(path)],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
            time.sleep(1.5);p.send_signal(signal.SIGINT);p.communicate(timeout=3)
            with wave.open(str(path)) as w:
                assert w.getframerate()==48000 and w.getnchannels()==1 and w.getsampwidth()==2
                data=w.readframes(w.getnframes())
            values=struct.unpack('<'+'h'*(len(data)//2),data)[12000:]
            assert len(values)>12000
            rms=math.sqrt(sum(x*x for x in values)/len(values))/32768
            crossings=sum(a<=0<b for a,b in zip(values,values[1:]))
            frequency=crossings*48000/len(values)
            return rms,frequency
        try:
            idle=wait(lambda s:s['pipewire_connected']);node_id=idle['node_id']
            p=sender();wait(lambda s:s['session_packets']>30)
            rms,freq=record();assert .08<rms<.3,(rms,freq);assert abs(freq-440)<8,(rms,freq)
            cli('mute');wait(lambda s:s['muted']);time.sleep(.15)
            silence,_=record();assert silence==0,silence
            cli('unmute');wait(lambda s:not s['muted']);time.sleep(.15)
            restored,_=record();assert restored>.08,restored
            # Abrupt network/control loss must preserve the exact PipeWire node and output silence.
            p.kill();p.communicate();wait(lambda s:s['state']=='IDLE');silent,_=record();assert silent==0
            assert cli('status')['node_id']==node_id
            p=sender('--loss-every','20');stats=wait(lambda s:s['state']=='STREAMING' and s['session_packets']>100)
            assert stats['packets_lost']>0 and stats['plc_frames']>0,stats
            lossy,_=record();assert lossy>.05
            assert cli('status')['node_id']==node_id
            p.kill();p.communicate();wait(lambda s:s['state']=='IDLE')
            # Unauthenticated LAN traffic cannot activate or alter a session.
            with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as udp:
                for n in [0,1,39,40,1200,1201,65507]: udp.sendto(bytes(n),('127.0.0.1',audio))
            time.sleep(.2);assert cli('status')['state']=='IDLE'
            result={'passed':True,'tone_rms':rms,'tone_hz':freq,'mute_rms':silence,'disconnect_rms':silent,'lossy_rms':lossy,'stable_node_id':node_id,'checks':['TLS pairing','AEAD Opus UDP','PipeWire recording','mute/unmute','abrupt disconnect silence','persistent source','reconnect','5% loss / PLC','malformed datagrams']}
            (ROOT/'artifacts').mkdir(exist_ok=True)
            (ROOT/'artifacts/integration.json').write_text(json.dumps(result,indent=2)+'\n')
            print(json.dumps(result,indent=2))
        finally:
            for p in senders:
                if p.poll() is None:p.kill();p.communicate()
            daemon.send_signal(signal.SIGINT)
            try:daemon.wait(timeout=8)
            except subprocess.TimeoutExpired:daemon.kill();daemon.wait()
            log.close()
if __name__=='__main__':main()
