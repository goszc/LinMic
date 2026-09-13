#pragma once
#include <oboe/Oboe.h>
#include <opus.h>
#include <mbedtls/chachapoly.h>
#include <atomic>
#include <thread>
#include <string>
#include "PcmRing.h"
#include "Dynamics.h"
extern std::atomic<bool> linmicPrivacyMute;
class AudioEngine final: public oboe::AudioStreamDataCallback, public oboe::AudioStreamErrorCallback {
 PcmRing<8192> ring;
 Dynamics dynamics;
 std::atomic<size_t> waveCursor{0};
 std::shared_ptr<oboe::AudioStream> stream;
 std::thread worker;
 OpusEncoder *encoder=nullptr;
 mbedtls_chachapoly_context crypto;
 int socketFd=-1,frame=480,bitrate=48000,deviceId=0;
 uint32_t session=0;
 bool compatible=true;
 void encodeLoop();
public:
 std::atomic<bool> running{false},muted{false};
 std::atomic<float> gainDb{0},rms{0},peak{0},rawRms{0},rawPeak{0};
 std::atomic<int> loss{0},error{0};
 std::atomic<uint64_t> callbacks{0},captureFrames{0},overruns{0},packets{0},sendErrors{0},encodeErrors{0},encodeUs{0};
 std::array<std::atomic<float>,128> waveform{};
 AudioEngine();
 ~AudioEngine();
 int configure(const std::string&,int,int,int,uint32_t,const uint8_t*,int,bool);
 int sessionId() const { return stream?static_cast<int>(stream->getSessionId()):0; }
 int start();
 void stop();
 oboe::DataCallbackResult onAudioReady(oboe::AudioStream*,void*,int32_t)override;
 void onErrorAfterClose(oboe::AudioStream*,oboe::Result result)override;
 void snapshot(float*,size_t);
};
