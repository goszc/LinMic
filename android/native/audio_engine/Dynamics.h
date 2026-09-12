#pragma once
#include <algorithm>
#include <cmath>
// Sample-by-sample gain ramp and peak limiter. Runs in the encoder worker, never the callback.
class Dynamics {
    float gain=1.f, limiter=1.f, cachedDb=0.f, targetGain=1.f;
public:
    float process(float input,float gainDb,bool muted) {
        if(gainDb!=cachedDb){cachedDb=gainDb;targetGain=std::pow(10.f,std::clamp(gainDb,-60.f,24.f)/20.f);}
        gain+=0.002f*(targetGain-gain); // about 10 ms smoothing at 48 kHz
        if(muted)return 0.f;
        const float amplified=input*gain;
        constexpr float ceiling=0.89125094f; // -1 dBFS
        const float required=std::min(1.f,ceiling/std::max(std::abs(amplified),ceiling));
        if(required<limiter)limiter=required;
        else limiter+=0.00026f*(required-limiter); // approximately 80 ms release
        return std::clamp(amplified*limiter,-ceiling,ceiling);
    }
};
