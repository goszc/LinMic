#pragma once
#include <array>
#include <atomic>
#include <cstdint>
#include <cstddef>
// Single producer callback, single consumer worker. Never advance the other index.
template<size_t N> class PcmRing {
 std::array<int16_t,N> data{};
 alignas(64) std::atomic<uint64_t> write{0};
 alignas(64) std::atomic<uint64_t> read{0};
public:
 bool push(const int16_t *input,size_t count) {
  auto w=write.load(std::memory_order_relaxed),r=read.load(std::memory_order_acquire);
  if(count>N-(w-r))return false;
  for(size_t i=0;i<count;i++)data[(w+i)%N]=input[i];
  write.store(w+count,std::memory_order_release);return true;
 }
 bool pop(int16_t *output,size_t count) {
  auto r=read.load(std::memory_order_relaxed),w=write.load(std::memory_order_acquire);
  if(w-r<count)return false;
  for(size_t i=0;i<count;i++)output[i]=data[(r+i)%N];
  read.store(r+count,std::memory_order_release);return true;
 }
 void clear(){read.store(write.load(std::memory_order_acquire),std::memory_order_release);}
 size_t available()const{return write.load(std::memory_order_acquire)-read.load(std::memory_order_acquire);}
};
