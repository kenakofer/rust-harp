package com.rustharp.app;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

public class MainActivity extends Activity {
    static {
        System.loadLibrary("rust_harp");
    }

    public static native int rustInit();

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        int v = rustInit();
        TextView tv = new TextView(this);
        tv.setText("Rust Harp (JNI loaded): " + v);
        setContentView(tv);
    }
}
