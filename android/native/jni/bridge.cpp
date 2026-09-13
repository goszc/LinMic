#include <cmath>
#include <algorithm>
#include <jni.h>
#include "../audio_engine/AudioEngine.h"
extern "C" {
JNIEXPORT void JNICALL Java_org_linmic_app_NativeAudio_setPrivacyMute(JNIEnv*,jobject,jboolean muted){linmicPrivacyMute.store(muted);}
JNIEXPORT jlong JNICALL Java_org_linmic_app_NativeAudio_create(JNIEnv*,jobject){return reinterpret_cast<jlong>(new AudioEngine());}
JNIEXPORT void JNICALL Java_org_linmic_app_NativeAudio_destroy(JNIEnv*,jobject,jlong h){delete reinterpret_cast<AudioEngine*>(h);}
JNIEXPORT jint JNICALL Java_org_linmic_app_NativeAudio_configure(JNIEnv* e,jobject,jlong h,jstring host,jint port,jint bitrate,jint frame,jlong session,jbyteArray key,jint device,jboolean compatible){
 if(!h||!host||!key||e->GetArrayLength(key)!=32)return -1;
 auto engine=reinterpret_cast<AudioEngine*>(h);const char* chars=e->GetStringUTFChars(host,nullptr);if(!chars)return -1;
 std::string hostname(chars);e->ReleaseStringUTFChars(host,chars);uint8_t bytes[32];e->GetByteArrayRegion(key,0,32,reinterpret_cast<jbyte*>(bytes));
 return engine->configure(hostname,port,bitrate,frame,static_cast<uint32_t>(session),bytes,device,compatible);
}
JNIEXPORT jint JNICALL Java_org_linmic_app_NativeAudio_sessionId(JNIEnv*,jobject,jlong h){return h?reinterpret_cast<AudioEngine*>(h)->sessionId():0;}
JNIEXPORT jint JNICALL Java_org_linmic_app_NativeAudio_start(JNIEnv*,jobject,jlong h){return h?reinterpret_cast<AudioEngine*>(h)->start():-1;}
JNIEXPORT void JNICALL Java_org_linmic_app_NativeAudio_stop(JNIEnv*,jobject,jlong h){if(h)reinterpret_cast<AudioEngine*>(h)->stop();}
JNIEXPORT void JNICALL Java_org_linmic_app_NativeAudio_setMuted(JNIEnv*,jobject,jlong h,jboolean muted){if(h)reinterpret_cast<AudioEngine*>(h)->muted.store(muted);}
JNIEXPORT void JNICALL Java_org_linmic_app_NativeAudio_setGain(JNIEnv*,jobject,jlong h,jfloat gain){if(h&&std::isfinite(gain))reinterpret_cast<AudioEngine*>(h)->gainDb.store(std::clamp(gain,-60.f,24.f));}
JNIEXPORT void JNICALL Java_org_linmic_app_NativeAudio_setLoss(JNIEnv*,jobject,jlong h,jint loss){if(h)reinterpret_cast<AudioEngine*>(h)->loss.store(loss);}
JNIEXPORT void JNICALL Java_org_linmic_app_NativeAudio_stats(JNIEnv* e,jobject,jlong h,jfloatArray array){if(!h||!array)return;jfloat data[148]{};reinterpret_cast<AudioEngine*>(h)->snapshot(data,148);e->SetFloatArrayRegion(array,0,std::min(148,e->GetArrayLength(array)),data);}
}
extern "C" {
void* linmic_pair_start(const uint8_t*,uint8_t*);
int linmic_pair_finish(void*,const uint8_t*,const uint8_t*,uint8_t*);
JNIEXPORT jbyteArray JNICALL Java_org_linmic_app_NativeAudio_pair(JNIEnv* env,jobject,jstring code,jbyteArray peer,jbyteArray certificate) {
 if(!code||!peer||!certificate||env->GetStringUTFLength(code)!=6||env->GetArrayLength(peer)!=33||env->GetArrayLength(certificate)!=32)return nullptr;
 uint8_t result[97]{},message[33]{},cert[32]{};
 env->GetByteArrayRegion(peer,0,33,reinterpret_cast<jbyte*>(message));
 env->GetByteArrayRegion(certificate,0,32,reinterpret_cast<jbyte*>(cert));
 const char* password=env->GetStringUTFChars(code,nullptr);if(!password)return nullptr;
 void* state=linmic_pair_start(reinterpret_cast<const uint8_t*>(password),result);
 env->ReleaseStringUTFChars(code,password);
 if(!state||linmic_pair_finish(state,message,cert,result+33)!=0)return nullptr;
 jbyteArray output=env->NewByteArray(97);if(output)env->SetByteArrayRegion(output,0,97,reinterpret_cast<jbyte*>(result));return output;
}
}
