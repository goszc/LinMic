#!/usr/bin/env python3
"""Package only committed source, preserving a reproducible file list and timestamps."""
import gzip,hashlib,io,pathlib,subprocess,tarfile
root=pathlib.Path(__file__).resolve().parents[1];version='0.3.0';out=root/'artifacts';out.mkdir(exist_ok=True)
if subprocess.check_output(['git','status','--porcelain','--untracked-files=normal'],cwd=root).strip():raise SystemExit('Commit reviewed source before packaging')
raw=subprocess.check_output(['git','archive','--format=tar','--prefix=linmic-'+version+'/','HEAD'],cwd=root)
path=out/f'linmic-{version}.tar.gz'
with path.open('wb') as f:
    with gzip.GzipFile(filename='',mode='wb',fileobj=f,mtime=0) as z:z.write(raw)
digest=hashlib.sha256(path.read_bytes()).hexdigest()
recipe=(root/'linux/packaging/arch/PKGBUILD.in').read_text().replace('@SOURCE_SHA256@',digest)
(out/'PKGBUILD').write_text(recipe)
print(path)
