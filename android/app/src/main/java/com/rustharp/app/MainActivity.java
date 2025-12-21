package com.rustharp.app;

import android.app.Activity;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.os.Build;
import android.os.Bundle;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.util.DisplayMetrics;
import android.util.Log;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.ViewGroup;
import android.widget.ImageView;

import java.util.ArrayList;
import java.util.List;

public class MainActivity extends Activity {
    static {
        System.loadLibrary("rust_harp");
    }

    private long rustHandle = 0;

    private int w;
    private int h;
    private int[] pixels;
    private Bitmap bmp;
    private ImageView iv;

    private RustAudio audio;
    private Vibrator vibrator;

    public static native int rustInit();
    public static native long rustCreateFrontend();
    public static native void rustDestroyFrontend(long handle);
    public static native int rustHandleAndroidKey(long handle, int keyCode, int unicodeChar, boolean isDown);
    public static native int rustHandleTouch(long handle, long pointerId, int phase, int x, int width);

    public static native void rustSetAudioSampleRate(long handle, int sampleRateHz);
    public static native int rustFillAudio(long handle, int frames, short[] outPcm);

    public static native boolean rustStartAAudio(long handle);
    public static native void rustStopAAudio(long handle);

    public static native void rustRenderStrings(long handle, int width, int height, int[] outPixels);

    private void redraw() {
        if (pixels == null || bmp == null || iv == null) {
            return;
        }
        rustRenderStrings(rustHandle, w, h, pixels);
        try {
            bmp.setPixels(pixels, 0, w, 0, 0, w, h);
            iv.invalidate();
        } catch (IllegalStateException e) {
            Log.e("RustHarp", "Bitmap.setPixels failed; recreating bitmap", e);
            bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888);
            bmp.setPixels(pixels, 0, w, 0, 0, w, h);
            iv.setImageBitmap(bmp);
            iv.invalidate();
        }
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        rustInit(); // smoke test: ensures JNI wiring is alive
        rustHandle = rustCreateFrontend();

        DisplayMetrics dm = getResources().getDisplayMetrics();
        w = dm.widthPixels;
        h = dm.heightPixels;

        pixels = new int[w * h];
        rustRenderStrings(rustHandle, w, h, pixels);

        // Use a mutable bitmap: createBitmap(int[]...) can yield an immutable bitmap on some devices.
        bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888);
        bmp.setPixels(pixels, 0, w, 0, 0, w, h);

        iv = new ImageView(this);
        iv.setImageBitmap(bmp);
        iv.setScaleType(ImageView.ScaleType.FIT_XY);
        iv.setBackgroundColor(0xFF000000);
        iv.setLayoutParams(new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT));
        iv.setPadding(0, 0, 0, 0);

        // Ensure we can receive hardware keyboard events.
        iv.setFocusable(true);
        iv.setFocusableInTouchMode(true);

        iv.setOnTouchListener((v, e) -> {
            if (rustHandle == 0) return true;

            int action = e.getActionMasked();
            int idx = e.getActionIndex();

            if (action == MotionEvent.ACTION_MOVE) {
                for (int i = 0; i < e.getPointerCount(); i++) {
                    int flags = rustHandleTouch(rustHandle, e.getPointerId(i), 1, (int) e.getX(i), w);
                    if ((flags & 1) != 0) redraw();
                    if ((flags & 2) != 0) Log.d("RustHarp", "touch play_notes");
                    if ((flags & 4) != 0) vibrateTick();
                }
                return true;
            }

            long pid = e.getPointerId(idx);
            int phase;
            if (action == MotionEvent.ACTION_DOWN || action == MotionEvent.ACTION_POINTER_DOWN) {
                phase = 0;
            } else if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_POINTER_UP) {
                phase = 2;
            } else if (action == MotionEvent.ACTION_CANCEL) {
                phase = 3;
            } else {
                return true;
            }

            int flags = rustHandleTouch(rustHandle, pid, phase, (int) e.getX(idx), w);
            if ((flags & 1) != 0) redraw();
            if ((flags & 2) != 0) Log.d("RustHarp", "touch play_notes");
            if ((flags & 4) != 0) vibrateTick();
            return true;
        });

        setContentView(iv);
        iv.requestFocus();

        // Prevent system back/forward gestures from stealing edge swipes.
        // Note: Android limits the total excluded area; we only exclude thin edge strips.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            iv.addOnLayoutChangeListener((v, left, top, right, bottom, oldLeft, oldTop, oldRight, oldBottom) -> {
                updateGestureExclusion();
            });
            iv.post(this::updateGestureExclusion);
        }

        vibrator = (Vibrator) getSystemService(VIBRATOR_SERVICE);

        boolean aaudioOk = rustStartAAudio(rustHandle);
        if (!aaudioOk) {
            audio = new RustAudio(rustHandle, (android.media.AudioManager) getSystemService(AUDIO_SERVICE));
            audio.start();
        } else {
            Log.i("RustHarp", "AAudio started");
        }
    }

    private int dpToPx(int dp) {
        return Math.round(dp * getResources().getDisplayMetrics().density);
    }

    private void updateGestureExclusion() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q || iv == null) return;

        int width = iv.getWidth();
        int height = iv.getHeight();
        if (width <= 0 || height <= 0) return;

        int edgePx = dpToPx(48);
        int leftEdge = Math.min(edgePx, width);
        int rightEdgeStart = Math.max(0, width - edgePx);

        List<Rect> rects = new ArrayList<>();
        rects.add(new Rect(0, 0, leftEdge, height));
        rects.add(new Rect(rightEdgeStart, 0, width, height));
        iv.setSystemGestureExclusionRects(rects);
    }

    private void vibrateTick() {
        if (vibrator == null || !vibrator.hasVibrator()) return;
        // Shortest reliable tick tends to be ~10ms.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            vibrator.vibrate(VibrationEffect.createOneShot(10, VibrationEffect.DEFAULT_AMPLITUDE));
        } else {
            vibrator.vibrate(10);
        }
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        boolean isDown = event.getAction() == KeyEvent.ACTION_DOWN;

        // Lowercase helps match the desktop mapping (which uses characters like 'a', 'd', etc.)
        int uc = event.getUnicodeChar();
        if (uc != 0) {
            uc = Character.toLowerCase((char) uc);
        }

        Log.d("RustHarp", "key action=" + event.getAction()
                + " keyCode=" + event.getKeyCode()
                + " scanCode=" + event.getScanCode()
                + " meta=" + event.getMetaState()
                + " uc=" + uc);

        int flags = 0;
        if (rustHandle != 0) {
            flags = rustHandleAndroidKey(rustHandle, event.getKeyCode(), uc, isDown);
            if ((flags & 1) != 0) {
                redraw();
            }
            if ((flags & 2) != 0) {
                Log.d("RustHarp", "key play_notes");
            }
        }

        // If Rust recognized the key at all (redraw and/or play_notes), consume it.
        // Falling through to the default handler can trigger system navigation/search behaviors
        // (e.g., KEYCODE_BACK / assist / launcher shortcuts) on some keyboards.
        if (flags != 0) {
            return true;
        }

        return super.dispatchKeyEvent(event);
    }

    @Override
    protected void onDestroy() {
        if (audio != null) {
            audio.stop();
            audio = null;
        }
        if (rustHandle != 0) {
            rustStopAAudio(rustHandle);
            rustDestroyFrontend(rustHandle);
            rustHandle = 0;
        }
        super.onDestroy();
    }
}
