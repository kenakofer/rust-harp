package com.rustharp.app;

import android.media.AudioAttributes;
import android.media.AudioFormat;
import android.media.AudioManager;
import android.media.AudioTrack;
import android.os.Build;
import android.os.Process;
import android.util.Log;

public final class RustAudio {
    private static final int CHANNEL_MASK = AudioFormat.CHANNEL_OUT_MONO;
    private static final int ENCODING = AudioFormat.ENCODING_PCM_16BIT;

    private final long rustHandle;
    private final AudioManager audioManager;

    private AudioTrack track;
    private Thread thread;
    private volatile boolean running = false;

    public RustAudio(long rustHandle, AudioManager audioManager) {
        this.rustHandle = rustHandle;
        this.audioManager = audioManager;
    }

    private static int parseIntOr(String s, int fallback) {
        if (s == null) return fallback;
        try {
            return Integer.parseInt(s);
        } catch (NumberFormatException ignored) {
            return fallback;
        }
    }

    public void start() {
        if (running) return;
        running = true;

        int nativeRate = parseIntOr(
                audioManager != null ? audioManager.getProperty(AudioManager.PROPERTY_OUTPUT_SAMPLE_RATE) : null,
                48000);
        int nativeFramesPerBuffer = parseIntOr(
                audioManager != null ? audioManager.getProperty(AudioManager.PROPERTY_OUTPUT_FRAMES_PER_BUFFER) : null,
                256);

        MainActivity.rustSetAudioSampleRate(rustHandle, nativeRate);

        int minBytes = AudioTrack.getMinBufferSize(nativeRate, CHANNEL_MASK, ENCODING);
        // Use the platform minimum (cannot go below it). Using the native sample rate helps keep this low.
        int bufBytes = minBytes;

        Log.i("RustHarp", "audio nativeRate=" + nativeRate
                + " framesPerBuf=" + nativeFramesPerBuffer
                + " minBytes=" + minBytes
                + " bufBytes=" + bufBytes);

        AudioAttributes attrs = new AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_GAME)
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .build();

        AudioFormat format = new AudioFormat.Builder()
                .setEncoding(ENCODING)
                .setSampleRate(nativeRate)
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
