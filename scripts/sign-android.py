#!/usr/bin/env python3
"""Build a release APK. Signing identity lives outside the repository, never in Gradle files."""
import json,os,pathlib,secrets,subprocess
root=pathlib.Path(__file__).resolve().parents[1]
secure=pathlib.Path.home()/'.local/share/linmic-signing'
secure.mkdir(parents=True,exist_ok=True,mode=0o700);secure.chmod(0o700)
credentials=secure/'credentials.json';keystore=secure/'release.jks'
if not credentials.exists():
    if keystore.exists():raise SystemExit('Existing keystore has no credentials file; restore your signing backup.')
    info={'password':secrets.token_urlsafe(40),'alias':'linmic-release'}
    fd=os.open(credentials,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    with os.fdopen(fd,'w') as f:json.dump(info,f)
info=json.loads(credentials.read_text());env=dict(os.environ)
env.update(LINMIC_KEYSTORE=str(keystore),LINMIC_STORE_PASSWORD=info['password'],LINMIC_KEY_PASSWORD=info['password'],LINMIC_KEY_ALIAS=info['alias'])
java=os.environ.get('JAVA_HOME','/usr/lib/jvm/java-17-openjdk');env['JAVA_HOME']=java
if not keystore.exists():
    subprocess.run([str(pathlib.Path(java)/'bin/keytool'),'-genkeypair','-keystore',str(keystore),'-storetype','JKS','-storepass:env','LINMIC_STORE_PASSWORD','-keypass:env','LINMIC_KEY_PASSWORD','-alias',info['alias'],'-keyalg','RSA','-keysize','4096','-sigalg','SHA256withRSA','-validity','10000','-dname','CN=LinMic Release, O=LinMic','-noprompt'],env=env,check=True)
    keystore.chmod(0o600)
subprocess.run([str(root/'android/gradlew'),'-p',str(root/'android'),':app:assembleRelease',':app:lintRelease',':app:testReleaseUnitTest'],env=env,check=True)
print('Release APK: android/app/build/outputs/apk/release/app-release.apk')
print('Back up ~/.local/share/linmic-signing securely; it is required for future compatible updates.')
