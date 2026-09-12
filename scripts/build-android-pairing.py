#!/usr/bin/env python3
"""Build the exact same PAKE implementation for all Android ABIs."""
import os,pathlib,shutil,subprocess
root=pathlib.Path(__file__).resolve().parents[1]
sdk=os.environ.get('ANDROID_HOME') or os.environ.get('ANDROID_SDK_ROOT')
if not sdk:
    for line in (root/'android/local.properties').read_text().splitlines():
        if line.startswith('sdk.dir='):sdk=line.split('=',1)[1]
if not sdk:raise SystemExit('Set ANDROID_HOME to the Android SDK directory')
ndk=pathlib.Path(sdk)/'ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin'
cargo=str(pathlib.Path.home()/'.cargo/bin/cargo') if (pathlib.Path.home()/'.cargo/bin/cargo').exists() else 'cargo'
for abi,target,compiler in [('arm64-v8a','aarch64-linux-android','aarch64-linux-android26-clang'),('armeabi-v7a','armv7-linux-androideabi','armv7a-linux-androideabi26-clang'),('x86_64','x86_64-linux-android','x86_64-linux-android26-clang')]:
    env=dict(os.environ);env['CARGO_TARGET_DIR']=str(root/'target/android');env['CARGO_TARGET_'+target.upper().replace('-','_')+'_LINKER']=str(ndk/compiler)
    env['RUSTFLAGS']='-C relocation-model=pic'
    subprocess.run([cargo,'build','--locked','--release','--target',target,'-p','linmic-pairing'],cwd=root,env=env,check=True)
    output=root/'android/native/rust'/abi;output.mkdir(parents=True,exist_ok=True)
    shutil.copy2(root/'target/android'/target/'release/liblinmic_pairing.a',output/'liblinmic_pairing.a')
