#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <portaudio.h>
#include <rnnoise.h>

#define SAMPLE_RATE 48000
#define FRAME_SIZE 480 // RNNoise ต้องการ 480 samples (10ms @ 48kHz)

typedef struct {
    DenoiseState *st;
    bool bypass;
    float vad_threshold;
} AudioContext;

// Callback ทำงานทุกๆ 10ms แบบ Real-time
static int audioCallback(const void *inputBuffer, void *outputBuffer,
                         unsigned long framesPerBuffer,
                         const PaStreamCallbackTimeInfo* timeInfo,
                         PaStreamCallbackFlags statusFlags,
                         void *userData) {
    AudioContext *ctx = (AudioContext *)userData;
    const float *in = (const float *)inputBuffer;
    float *out = (float *)outputBuffer;

    if (inputBuffer == NULL) {
        for (unsigned long i = 0; i < framesPerBuffer; i++) out[i] = 0.0f;
        return paContinue;
    }

    if (ctx->bypass) {
        for (unsigned long i = 0; i < framesPerBuffer; i++) out[i] = in[i];
        return paContinue;
    }

    // RNNoise คาดหวังค่า Audio ในช่วง [-32768.0, 32767.0] แต่ PortAudio ให้ค่า [-1.0, 1.0]
    float rnnoise_buf[FRAME_SIZE];
    for (int i = 0; i < FRAME_SIZE; i++) {
        rnnoise_buf[i] = in[i] * 32768.0f;
    }

    // ประมวลผลตัดเสียงรบกวน (คืนค่าเป็น Voice Activity Detection 0.0 - 1.0)
    float vad_prob = rnnoise_process_frame(ctx->st, rnnoise_buf, rnnoise_buf);

    // แปลงกลับเป็น [-1.0, 1.0] และใช้ VAD Gate ตัดเสียงรบกวนช่วงไม่พูด
    for (int i = 0; i < FRAME_SIZE; i++) {
        if (vad_prob >= ctx->vad_threshold) {
            out[i] = rnnoise_buf[i] / 32768.0f;
        } else {
            out[i] = 0.0f; // Mute เมื่อไม่มีเสียงพูด
        }
    }

    return paContinue;
}

// ฟังก์ชัน Export สำหรับให้ Rust (Tauri) เรียกใช้
__declspec(dllexport) int start_noise_cancel(int input_dev_idx, int output_dev_idx, float threshold) {
    Pa_Initialize();

    AudioContext *ctx = (AudioContext *)malloc(sizeof(AudioContext));
    ctx->st = rnnoise_create(NULL);
    ctx->bypass = false;
    ctx->vad_threshold = threshold;

    PaStreamParameters inputParams, outputParams;
    inputParams.device = input_dev_idx;
    inputParams.channelCount = 1;
    inputParams.sampleFormat = paFloat32;
    inputParams.suggestedLatency = Pa_GetDeviceInfo(input_dev_idx)->defaultLowInputLatency;
    inputParams.hostApiSpecificStreamInfo = NULL;

    outputParams.device = output_dev_idx; // เลือก index ของ CABLE Input
    outputParams.channelCount = 1;
    outputParams.sampleFormat = paFloat32;
    outputParams.suggestedLatency = Pa_GetDeviceInfo(output_dev_idx)->defaultLowOutputLatency;
    outputParams.hostApiSpecificStreamInfo = NULL;

    PaStream *stream;
    PaError err = Pa_OpenStream(&stream, &inputParams, &outputParams,
                                SAMPLE_RATE, FRAME_SIZE, paClipOff,
                                audioCallback, ctx);
    if (err != paNoError) return -1;

    Pa_StartStream(stream);
    return 0;
}