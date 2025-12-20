package com.rustharp.app;

import android.app.Activity;
import android.graphics.Bitmap;
import android.os.Bundle;
import android.util.DisplayMetrics;
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

    public static native int rustInit();
    public static native long rustCreateFrontend();
    public static native void rustDestroyFrontend(long handle);
    public static native int rustHandleAndroidKey(long handle, int keyCode, int unicodeChar, boolean isDown);
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
        setContentView(iv);
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (rustHandle != 0) {
            boolean isDown = event.getAction() == KeyEvent.ACTION_DOWN;
            // Lowercase helps match the desktop mapping (which uses characters like 'a', 'd', etc.)
            int uc = event.getUnicodeChar();
            if (uc != 0) {
                uc = Character.toLowerCase((char) uc);
            }

            int flags = rustHandleAndroidKey(rustHandle, event.getKeyCode(), uc, isDown);
            if ((flags & 1) != 0) {
                redraw();
            }
        }
        return super.dispatchKeyEvent(event);
    }

    @Override
    protected void onDestroy() {
        if (rustHandle != 0) {
            rustDestroyFrontend(rustHandle);
            rustHandle = 0;
        }
        super.onDestroy();
    }
}
