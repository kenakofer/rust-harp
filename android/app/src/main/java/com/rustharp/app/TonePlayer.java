package com.rustharp.app;

import android.media.AudioAttributes;
import android.media.AudioFormat;
import android.media.AudioManager;
import android.media.AudioTrack;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class TonePlayer {
    private static final int SAMPLE_RATE = 48000;
    private static final float DURATION_SECONDS = 1.0f;

    private final ExecutorService exec = Executors.newSingleThreadExecutor();

    public void playSquare(final float freqHz, final int volume0to127) {
        exec.execute(() -> {
            int samples = (int) (SAMPLE_RATE * DURATION_SECONDS);
            short[] pcm = new short[samples];

            // Conservative amplitude to avoid clipping/painful loudness.
            float amp = (volume0to127 / 127.0f) * 0.2f;
            short hi = (short) (amp * Short.MAX_VALUE);
            short lo = (short) (-hi);

            double phase = 0.0;
            double phaseInc = (2.0 * Math.PI * freqHz) / SAMPLE_RATE;
            for (int i = 0; i < samples; i++) {
                pcm[i] = (Math.sin(phase) >= 0.0) ? hi : lo;
                phase += phaseInc;
                if (phase > 2.0 * Math.PI) {
                    phase -= 2.0 * Math.PI;
                }
            }

            AudioTrack track = new AudioTrack(
                    new AudioAttributes.Builder()
                            .setLegacyStreamType(AudioManager.STREAM_MUSIC)
                            .build(),
                    new AudioFormat.Builder()
                            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                            .setSampleRate(SAMPLE_RATE)
                            .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                            .build(),
                    pcm.length * 2,
                    AudioTrack.MODE_STATIC,
                    AudioManager.AUDIO_SESSION_ID_GENERATE
            );

            track.write(pcm, 0, pcm.length);
            track.play();

            // Busy-wait is avoided; just sleep slightly longer than duration.
            try {
                Thread.sleep((long) (DURATION_SECONDS * 1000.0f) + 20L);
            } catch (InterruptedException ignored) {
            }

            track.stop();
            track.release();
        });
    }

    public void shutdown() {
        exec.shutdownNow();
    }
}
