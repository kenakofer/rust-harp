package com.rustharp.app;

import android.app.Activity;
import android.graphics.Bitmap;
import android.os.Bundle;
import android.util.DisplayMetrics;
import android.util.Log;
import android.view.KeyEvent;
import android.view.ViewGroup;
import android.widget.ImageView;

public class MainActivity extends Activity {
    static {
        System.loadLibrary("rust_harp");
    }

    private long rustHandle = 0;

    private int w;
    private int h;
    private int[] pixels;
    private Bitmap bmp;

    private RustAudio audio;

    public static native int rustInit();
    public static native long rustCreateFrontend();
    public static native void rustDestroyFrontend(long handle);
    public static native int rustHandleAndroidKey(long handle, int keyCode, int unicodeChar, boolean isDown);

    public static native void rustSetAudioSampleRate(long handle, int sampleRateHz);
    public static native int rustFillAudio(long handle, int frames, short[] outPcm);

    public static native void rustRenderStrings(long handle, int width, int height, int[] outPixels);

    private void redraw() {
        if (pixels == null || bmp == null) {
            return;
        }
        rustRenderStrings(rustHandle, w, h, pixels);
        bmp.setPixels(pixels, 0, w, 0, 0, w, h);
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

        bmp = Bitmap.createBitmap(pixels, w, h, Bitmap.Config.ARGB_8888);
        ImageView iv = new ImageView(this);
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

        setContentView(iv);
        iv.requestFocus();

        audio = new RustAudio(rustHandle, 48000);
        audio.start();
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
            rustDestroyFrontend(rustHandle);
            rustHandle = 0;
        }
        super.onDestroy();
    }
}
