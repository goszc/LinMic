#include "../../android/native/audio_engine/PcmRing.h"
#include "../../android/native/network/Packetizer.h"
#include <cassert>
#include "../../android/native/audio_engine/Dynamics.h"
#include <thread>
#include <iostream>
int main(){
 PcmRing<4> r;int16_t in[]={1,2,3,4},out[4]{};assert(r.push(in,4));assert(!r.push(in,1));assert(r.pop(out,4));for(int i=0;i<4;i++)assert(out[i]==in[i]);assert(!r.pop(out,1));
 PcmRing<1024> threaded;std::thread producer([&]{for(int i=0;i<1000000;i++){int16_t v=i%30000;while(!threaded.push(&v,1))std::this_thread::yield();}});for(int i=0;i<1000000;i++){int16_t v;while(!threaded.pop(&v,1))std::this_thread::yield();assert(v==i%30000);}producer.join();
 uint8_t p[40]{};linmic::header(p,16,123,0,0,9,480,3);assert(p[0]=='L'&&p[3]=='C'&&p[4]==1&&p[5]==1);assert(p[11]==123);assert(p[31]==9);assert(p[32]==1&&p[33]==224);assert(p[35]==3);for(int i=36;i<40;i++)assert(p[i]==0);
 auto n=linmic::nonce(0x12345678,0xabcdef01);assert(n[0]==0x12&&n[3]==0x78&&n[4]==0xab&&n[7]==1&&n[11]==0);
 Dynamics boost;float steady=0;for(int i=0;i<48000;i++)steady=boost.process(0.05f,12.f,false);assert(std::abs(steady-0.19905f)<0.001f);
 for(int i=0;i<48000;i++)assert(std::abs(boost.process(i%2?1.f:-1.f,24.f,false))<=0.892f);
 assert(boost.process(1.f,24.f,true)==0.f);
 std::cout<<"Native ring concurrency and wire vectors passed\n";
}
