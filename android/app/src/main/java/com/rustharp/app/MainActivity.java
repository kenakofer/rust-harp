package com.rustharp.app;

import android.app.Activity;
import android.graphics.Bitmap;
import android.os.Bundle;
import android.util.DisplayMetrics;
import android.view.ViewGroup;
import android.widget.ImageView;

public class MainActivity extends Activity {
    static {
        System.loadLibrary("rust_harp");
    }

    public static native int rustInit();
    public static native void rustRenderStrings(int width, int height, int[] outPixels);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        rustInit(); // smoke test: ensures JNI wiring is alive

        DisplayMetrics dm = getResources().getDisplayMetrics();
        int w = dm.widthPixels;
        int h = dm.heightPixels;

        int[] pixels = new int[w * h];
        rustRenderStrings(w, h, pixels);

        Bitmap bmp = Bitmap.createBitmap(pixels, w, h, Bitmap.Config.ARGB_8888);
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
}
