#!/usr/bin/env python3
import pathlib,shutil,subprocess,tarfile
root=pathlib.Path(__file__).resolve().parents[1]
version='0.3.0';dest=root/'artifacts';folder=dest/f'linmic-{version}-linux-x86_64';folder.mkdir(parents=True,exist_ok=True)
(folder/'bin').mkdir(exist_ok=True)
for name in ['linmicd','linmic','linmic-gui','linmic-hotkeys']:
    shutil.copy2(root/'target/release'/name,folder/'bin'/name)
for name in ['linux','scripts','docs']:
    if (folder/name).exists():shutil.rmtree(folder/name)
    if name=='linux':
        (folder/name/'systemd').mkdir(parents=True)
        for file in ['linmic.desktop','linmic.svg','systemd/linmic.service']:shutil.copy2(root/'linux'/file,folder/'linux'/file)
    elif name=='scripts':(folder/name).mkdir();shutil.copy2(root/'scripts/install-linux.sh',folder/'scripts/install-linux.sh')
    else:shutil.copytree(root/name,folder/name)
for name in ['README.md','LICENSE','SECURITY.md','THIRD_PARTY_NOTICES.md']:shutil.copy2(root/name,folder/name)
subprocess.run(['tar','--zstd','-cf',str(dest/f'{folder.name}.tar.zst'),'-C',str(dest),folder.name],check=True)
print(dest/f'{folder.name}.tar.zst')
