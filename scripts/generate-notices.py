#!/usr/bin/env python3
"""Collect license texts from locked dependencies; run after an Android native build."""
import json,pathlib,subprocess,urllib.request,shutil
root=pathlib.Path(__file__).resolve().parents[1]
m=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=root))
out=root/'docs/licenses';out.mkdir(parents=True,exist_ok=True)
rows=['# Third-party notices\n','LinMic is GPL-3.0-or-later. Dependencies retain the licenses below. This inventory includes build and platform-specific dependencies, not all of which are linked into every binary. When a choice is offered, distribution uses a GPL-compatible permissive option. Full supplied notices are included under `docs/licenses`.\n','| Component | License | Texts |','| --- | --- | --- |']
android_tree=subprocess.check_output(['cargo','tree','--locked','-p','linmic-pairing','--target','aarch64-linux-android','--prefix','none','--format','{p}'],cwd=root,text=True)
android_names=set(line.split()[0] for line in android_tree.splitlines() if line.strip())
android_text=['LinMic — GPL-3.0-or-later\nCopyright 2026 LinMic contributors.\n', (root/'LICENSE').read_text()]
missing=[]
for p in sorted(m['packages'],key=lambda p:(p['name'],p['version'])):
    if not p['source']:continue
    base=pathlib.Path(p['manifest_path']).parent;dest=out/(p['name']+'-'+p['version']);files=[]
    for f in base.rglob('*'):
        if f.is_file() and f.suffix not in ('.rs','.c','.h','.cpp') and (f.name.lower().startswith(('license','licence','copying','copyright','notice','unlicense','authors')) or any(x.lower() in ('licenses','licences') for x in f.relative_to(base).parts[:-1])):
            if f.stat().st_size>300000:continue
            rel=f.relative_to(base);target=dest/rel;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(f,target);files.append(f)
    rows.append(f"| {p['name']} {p['version']} | {p['license']} | [notices](docs/licenses/{dest.name}) |")
    if not files:
        vcs=json.loads((base/'.cargo_vcs_info.json').read_text())['git']['sha1']
        repo=p['repository'].removeprefix('https://github.com/').removesuffix('.git')
        tree=json.loads(subprocess.check_output(['gh','api',f'repos/{repo}/git/trees/{vcs}?recursive=1']))
        for entry in tree['tree']:
            path=entry['path']
            if entry['type']=='blob' and pathlib.PurePosixPath(path).suffix not in ('.rs','.c','.h','.cpp') and pathlib.PurePosixPath(path).name.lower().startswith(('license','licence','copying','unlicense')):
                target=dest/path;target.parent.mkdir(parents=True,exist_ok=True)
                target.write_bytes(urllib.request.urlopen(f'https://raw.githubusercontent.com/{repo}/{vcs}/{path}').read());files.append(target)
        if not files:missing.append(p['name'])
    if p['name'] in android_names:
        android_text.append('\n\n=== '+p['name']+' '+p['version']+' — '+p['license']+' ===\n')
        android_text.extend(f.read_text(errors='replace') for f in files)
if missing:raise SystemExit('Missing license files: '+', '.join(missing))
for name,version,license,url in [
 ('oboe','1.10.0','Apache-2.0','https://raw.githubusercontent.com/google/oboe/1.10.0/LICENSE'),
 ('rust','1.98.1','MIT OR Apache-2.0','https://raw.githubusercontent.com/rust-lang/rust/1.98.1/LICENSE-MIT'),
 ('rust-apache','1.98.1','Apache-2.0','https://raw.githubusercontent.com/rust-lang/rust/1.98.1/LICENSE-APACHE')]:
    dest=out/(name+'-'+version);dest.mkdir(exist_ok=True)
    text=urllib.request.urlopen(url).read().decode();(dest/'LICENSE').write_text(text)
    rows.append(f'| {name} {version} | {license} | [license](docs/licenses/{dest.name}/LICENSE) |')
    android_text.extend(['\n\n=== '+name+' '+version+' ===\n',text])
for name,version,license,filename in [('opus','1.5.2','BSD-3-Clause','COPYING'),('mbedtls','3.6.7','Apache-2.0 OR GPL-2.0-or-later','LICENSE')]:
    sources=list((root/'android/app/.cxx/RelWithDebInfo').glob('*/arm64-v8a/_deps/'+name+'-src/'+filename))
    if not sources:raise SystemExit('Build Android release first: missing '+name)
    dest=out/(name+'-'+version);dest.mkdir(exist_ok=True);text=sources[0].read_text();(dest/filename).write_text(text)
    rows.append(f'| {name} {version} | {license} | [license](docs/licenses/{dest.name}/{filename}) |')
    android_text.extend(['\n\n=== '+name+' '+version+' ===\n',text])
rows.append('\nLinux system libraries (PipeWire, GTK4, GLib, Opus and their dependencies) are dynamically linked and supplied by the distribution under their own licenses. Android libc++ uses Apache-2.0 WITH LLVM-exception; see the NDK notice included below.\n')
sdk=pathlib.Path(__import__('os').environ.get('ANDROID_HOME',str(root/'.tools/android-sdk')))
ndk=sdk/'ndk/28.2.13676358'
notice=ndk/'toolchains/llvm/prebuilt/linux-x86_64/NOTICE'
if notice.exists():
    dest=out/'android-libcxx';dest.mkdir(exist_ok=True);shutil.copy2(notice,dest/'NOTICE');android_text.extend(['\n\n=== Android libc++ ===\n',notice.read_text()])
else:raise SystemExit('Missing NDK libc++ NOTICE')
(root/'THIRD_PARTY_NOTICES.md').write_text('\n'.join(rows))
(root/'android/app/src/main/res/raw/licenses.txt').write_text('\n'.join(android_text))
print('Generated notices for',len(m['packages']),'packages; Android text',sum(map(len,android_text)),'characters')
