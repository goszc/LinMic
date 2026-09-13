#include "AudioEngine.h"
#include "../network/Packetizer.h"
#include <sys/socket.h>
#include <netdb.h>
#include <unistd.h>
#include <cmath>
#include <algorithm>
#include <chrono>
using Clock=std::chrono::steady_clock;
std::atomic<bool> linmicPrivacyMute{false};
AudioEngine::AudioEngine(){mbedtls_chachapoly_init(&crypto);for(auto &v:waveform)v.store(0);}
AudioEngine::~AudioEngine(){stop();mbedtls_chachapoly_free(&crypto);}
int AudioEngine::configure(const std::string& host,int port,int bits,int samples,uint32_t id,const uint8_t* key,int device,bool compatibility){
 if(running||port<1||port>65535||(samples!=240&&samples!=480&&samples!=960)||bits<6000||bits>128000)return -1;
 frame=samples;bitrate=bits;session=id;deviceId=device;compatible=compatibility;
 addrinfo hints{},*addresses=nullptr;hints.ai_family=AF_UNSPEC;hints.ai_socktype=SOCK_DGRAM;
 if(getaddrinfo(host.c_str(),std::to_string(port).c_str(),&hints,&addresses)!=0)return -2;
 for(auto a=addresses;a;a=a->ai_next){int fd=socket(a->ai_family,SOCK_DGRAM|SOCK_CLOEXEC,a->ai_protocol);if(fd<0)continue;if(connect(fd,a->ai_addr,a->ai_addrlen)==0){socketFd=fd;break;}close(fd);}
 freeaddrinfo(addresses);if(socketFd<0)return -3;
 timeval timeout{0,20000};setsockopt(socketFd,SOL_SOCKET,SO_SNDTIMEO,&timeout,sizeof(timeout));
 if(mbedtls_chachapoly_setkey(&crypto,key)!=0)return -4;
 return 0;
}
int AudioEngine::start(){
 if(running||socketFd<0)return -1;int result=0;
 encoder=opus_encoder_create(48000,1,frame==240?OPUS_APPLICATION_RESTRICTED_LOWDELAY:OPUS_APPLICATION_VOIP,&result);
 if(result!=OPUS_OK||!encoder){stop();return -5;}
 opus_encoder_ctl(encoder,OPUS_SET_BITRATE(bitrate));opus_encoder_ctl(encoder,OPUS_SET_COMPLEXITY(5));opus_encoder_ctl(encoder,OPUS_SET_DTX(0));
 oboe::AudioStreamBuilder builder;
 builder.setSessionId(oboe::SessionId::Allocate);
 builder.setDirection(oboe::Direction::Input)->setChannelCount(1)->setSampleRate(48000)->setFormat(oboe::AudioFormat::I16)->setFormatConversionAllowed(true)->setSampleRateConversionQuality(oboe::SampleRateConversionQuality::Medium)->setPerformanceMode(oboe::PerformanceMode::LowLatency)->setSharingMode(compatible?oboe::SharingMode::Shared:oboe::SharingMode::Exclusive)->setInputPreset(compatible?oboe::InputPreset::VoiceCommunication:oboe::InputPreset::VoiceRecognition)->setDataCallback(this)->setErrorCallback(this);
 if(deviceId!=0)builder.setDeviceId(deviceId);
 auto opened=builder.openStream(stream);if(opened!=oboe::Result::OK){builder.setSharingMode(oboe::SharingMode::Shared);opened=builder.openStream(stream);}
 if(opened!=oboe::Result::OK||!stream){stop();return static_cast<int>(opened);}
 if(stream->getSampleRate()!=48000||stream->getChannelCount()!=1||stream->getFormat()!=oboe::AudioFormat::I16){stop();return -6;}
 ring.clear();error.store(0);running.store(true);worker=std::thread(&AudioEngine::encodeLoop,this);
 auto started=stream->requestStart();if(started!=oboe::Result::OK){stop();return static_cast<int>(started);}return 0;
}
void AudioEngine::stop(){running.store(false);if(stream){stream->requestStop();stream->close();stream.reset();}if(worker.joinable())worker.join();if(encoder){opus_encoder_destroy(encoder);encoder=nullptr;}if(socketFd>=0){close(socketFd);socketFd=-1;}ring.clear();rms.store(0);peak.store(0);}
oboe::DataCallbackResult AudioEngine::onAudioReady(oboe::AudioStream*,void* data,int32_t frames){callbacks.fetch_add(1,std::memory_order_relaxed);captureFrames.fetch_add(frames,std::memory_order_relaxed);if(!ring.push(static_cast<int16_t*>(data),frames))overruns.fetch_add(1,std::memory_order_relaxed);return oboe::DataCallbackResult::Continue;}
void AudioEngine::onErrorAfterClose(oboe::AudioStream*,oboe::Result result){error.store(static_cast<int>(result));running.store(false);}
void AudioEngine::encodeLoop(){
 std::array<int16_t,960> pcm{};std::array<uint8_t,linmic::MaxPayload> encoded{};std::array<uint8_t,linmic::MaxDatagram> packet{};
 uint32_t sequence=0;uint64_t sample=0;size_t wave=0;const auto origin=Clock::now();
 while(running.load()){
  if(sequence==UINT32_MAX){error.store(-7);running.store(false);break;}
  if(ring.available()>static_cast<size_t>(frame*4)){ring.clear();overruns.fetch_add(1);}
  if(!ring.pop(pcm.data(),frame)){std::this_thread::sleep_for(std::chrono::milliseconds(1));continue;}
  const bool mute=muted.load()||linmicPrivacyMute.load();const float gain=gainDb.load();float sum=0,p=0,rawSum=0,rawP=0;
  for(int i=0;i<frame;i++){const float raw=pcm[i]/32768.f;rawSum+=raw*raw;rawP=std::max(rawP,std::abs(raw));float x=dynamics.process(raw,gain,mute);pcm[i]=static_cast<int16_t>(x*32767);sum+=x*x;p=std::max(p,std::abs(x));if(i%std::max(1,frame/8)==0)waveform[(wave++)%128].store(std::abs(x));}
  rawRms.store(std::sqrt(rawSum/frame));rawPeak.store(rawP);waveCursor.store(wave);rms.store(std::sqrt(sum/frame));peak.store(p);
  int expected=frame>=480?std::clamp(loss.load(),0,20):0;opus_encoder_ctl(encoder,OPUS_SET_INBAND_FEC(expected>0));opus_encoder_ctl(encoder,OPUS_SET_PACKET_LOSS_PERC(expected));
  auto before=Clock::now();int length=opus_encode(encoder,pcm.data(),frame,encoded.data(),encoded.size());encodeUs.store(std::chrono::duration_cast<std::chrono::microseconds>(Clock::now()-before).count());
  if(length<0){encodeErrors.fetch_add(1);continue;}
  uint16_t flags=16|(mute?1:0)|(expected>0?2:0)|(sequence==0?4:0);
  linmic::header(packet.data(),flags,session,sequence,sample,std::chrono::duration_cast<std::chrono::microseconds>(before-origin).count(),frame,length);
  auto nonce=linmic::nonce(session,sequence);
  if(mbedtls_chachapoly_encrypt_and_tag(&crypto,length,nonce.data(),packet.data(),40,encoded.data(),packet.data()+40,packet.data()+40+length)!=0){encodeErrors.fetch_add(1);break;}
  auto sent=send(socketFd,packet.data(),40+length+16,MSG_NOSIGNAL);if(sent!=40+length+16)sendErrors.fetch_add(1);else packets.fetch_add(1);
  sequence++;sample+=frame;
 }
}
void AudioEngine::snapshot(float* out,size_t n){if(n<16)return;out[0]=rms.load();out[1]=peak.load();out[2]=packets.load();out[3]=overruns.load();out[4]=sendErrors.load();out[5]=encodeUs.load()/1000.f;out[6]=error.load();out[7]=captureFrames.load();out[8]=stream?stream->getSampleRate():0;out[9]=stream?stream->getFramesPerBurst():0;out[10]=stream?static_cast<int>(stream->getAudioApi()):0;out[11]=stream?stream->getDeviceId():0;out[12]=stream && stream->getXRunCount() ? stream->getXRunCount().value() : 0;out[13]=callbacks.load();out[14]=stream?static_cast<int>(stream->getPerformanceMode()):0;out[15]=stream?static_cast<int>(stream->getSessionId()):0;if(n>=148){out[144]=rawRms.load();out[145]=rawPeak.load();out[146]=stream?static_cast<int>(stream->getState()):0;out[147]=running.load()?1.f:0.f;}for(size_t i=16;i<n&&i<144;i++)out[i]=waveform[(i-16+waveCursor.load())%128].load();}
