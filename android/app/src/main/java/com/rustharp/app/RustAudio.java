package com.rustharp.app;

import android.media.AudioAttributes;
import android.media.AudioFormat;
import android.media.AudioManager;
import android.media.AudioTrack;
import android.os.Build;
import android.os.Process;

public final class RustAudio {
    private static final int CHANNEL_MASK = AudioFormat.CHANNEL_OUT_MONO;
    private static final int ENCODING = AudioFormat.ENCODING_PCM_16BIT;

    private final long rustHandle;
    private final int sampleRate;

    private AudioTrack track;
    private Thread thread;
    private volatile boolean running = false;

    public RustAudio(long rustHandle, int sampleRate) {
        this.rustHandle = rustHandle;
        this.sampleRate = sampleRate;
    }

    public void start() {
        if (running) return;
        running = true;

        MainActivity.rustSetAudioSampleRate(rustHandle, sampleRate);

        int minBytes = AudioTrack.getMinBufferSize(sampleRate, CHANNEL_MASK, ENCODING);
        // Latency is largely buffered; start with the platform minimum.
        int bufBytes = minBytes;

        AudioAttributes attrs = new AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_MEDIA)
                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                .build();

        AudioFormat format = new AudioFormat.Builder()
                .setEncoding(ENCODING)
                .setSampleRate(sampleRate)
                .setChannelMask(CHANNEL_MASK)
                .build();

        AudioTrack.Builder b = new AudioTrack.Builder()
                .setAudioAttributes(attrs)
                .setAudioFormat(format)
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setBufferSizeInBytes(bufBytes);

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            b.setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY);
        }

        track = b.build();

        track.play();

        thread = new Thread(() -> {
            Process.setThreadPriority(Process.THREAD_PRIORITY_AUDIO);
            // frames = samples for mono
            int frames = bufBytes / 2;
            short[] pcm = new short[frames];
            while (running) {
                int written = MainActivity.rustFillAudio(rustHandle, frames, pcm);
                if (written > 0) {
                    track.write(pcm, 0, written);
                }
            }
        }, "RustAudio");
        thread.start();
    }

    public void stop() {
        running = false;
        if (thread != null) {
            try {
                thread.join(250);
            } catch (InterruptedException ignored) {
            }
            thread = null;
        }
        if (track != null) {
            track.stop();
            track.release();
            track = null;
        }
    }
}
