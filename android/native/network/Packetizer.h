#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
namespace linmic {
constexpr size_t HeaderSize=40,MaxDatagram=1200,MaxPayload=1144;
inline void be(uint8_t* p,uint64_t v,size_t n){for(size_t i=0;i<n;i++)p[n-1-i]=static_cast<uint8_t>(v>>(i*8));}
inline void header(uint8_t* p,uint16_t flags,uint32_t session,uint32_t sequence,uint64_t sample,uint64_t time,uint16_t frame,uint16_t length){
 p[0]='L';p[1]='M';p[2]='I';p[3]='C';p[4]=1;p[5]=1;be(p+6,flags,2);be(p+8,session,4);be(p+12,sequence,4);be(p+16,sample,8);be(p+24,time,8);be(p+32,frame,2);be(p+34,length,2);be(p+36,0,4);
}
inline std::array<uint8_t,12> nonce(uint32_t session,uint32_t sequence){std::array<uint8_t,12> n{};be(n.data(),session,4);be(n.data()+4,sequence,4);return n;}
}
