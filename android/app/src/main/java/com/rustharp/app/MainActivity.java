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
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.GridLayout;
import android.widget.ImageView;

import java.util.ArrayList;
import java.util.List;

public class MainActivity extends Activity {
    private static final int BTN_VIIB = 0;
    private static final int BTN_IV = 1;
    private static final int BTN_I = 2;
    private static final int BTN_V = 3;
    private static final int BTN_II = 4;
    private static final int BTN_IV_MINOR = 5;
    private static final int BTN_III = 6;
    private static final int BTN_VII_DIM = 7;

    private static final int BTN_MAJ7 = 8;
    private static final int BTN_NO3 = 9;
    private static final int BTN_SUS4 = 10;
    private static final int BTN_MM = 11;
    private static final int BTN_ADD2 = 12;
    private static final int BTN_ADD7 = 13;
    private static final int BTN_HEPT = 14;
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

    private Button[] uiButtons = new Button[15];

    public static native int rustInit();
    public static native long rustCreateFrontend();
    public static native void rustDestroyFrontend(long handle);
    public static native int rustHandleAndroidKey(long handle, int keyCode, int unicodeChar, boolean isDown);
    public static native int rustHandleUiButton(long handle, int buttonId, boolean isDown);
    public static native int rustGetUiButtonsMask(long handle);
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

    private void updateUiButtons() {
        if (rustHandle == 0) return;
        int mask = rustGetUiButtonsMask(rustHandle);
        for (int i = 0; i < uiButtons.length; i++) {
            Button b = uiButtons[i];
            if (b == null) continue;
            boolean on = (mask & (1 << i)) != 0;
            // Simple, high-contrast toggle.
            b.setBackgroundColor(on ? 0xFF444444 : 0xFF111111);
            b.setTextColor(on ? 0xFFFFFFFF : 0xFFCCCCCC);
        }
    }

    private Button makeUiButton(String label, int id, int wPx, int hPx) {
        Button b = new Button(this);
        b.setText(label);
        b.setAllCaps(false);
        b.setPadding(0, 0, 0, 0);
        b.setMinWidth(0);
        b.setMinHeight(0);

        GridLayout.LayoutParams lp = new GridLayout.LayoutParams();
        lp.width = wPx;
        lp.height = hPx;
        lp.setMargins(0, 0, 0, 0);
        b.setLayoutParams(lp);

        b.setOnTouchListener((v, e) -> {
            if (rustHandle == 0 || id < 0) return true;
            int action = e.getActionMasked();
            if (action == MotionEvent.ACTION_DOWN || action == MotionEvent.ACTION_POINTER_DOWN) {
                rustHandleUiButton(rustHandle, id, true);
                redraw();
                updateUiButtons();
                return true;
            }
            if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_POINTER_UP || action == MotionEvent.ACTION_CANCEL) {
                rustHandleUiButton(rustHandle, id, false);
                redraw();
                updateUiButtons();
                return true;
            }
            return true;
        });

        return b;
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

        FrameLayout root = new FrameLayout(this);
        root.setLayoutParams(new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT));
        root.setMotionEventSplittingEnabled(true);

        root.addView(iv);

        // Touch chord/modifier grid (lower-left).
        GridLayout grid = new GridLayout(this);
        grid.setColumnCount(4);
        grid.setRowCount(4);
        grid.setUseDefaultMargins(false);
        grid.setPadding(0, 0, 0, 0);
        grid.setMotionEventSplittingEnabled(true);

        FrameLayout.LayoutParams glp = new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT);
        glp.leftMargin = 0;
        glp.bottomMargin = 0;
        glp.gravity = android.view.Gravity.BOTTOM | android.view.Gravity.START;
        grid.setLayoutParams(glp);

        int bw = dpToPx(64);
        int bh = dpToPx(42);

        // Row 1: VIIb IV I V
        uiButtons[BTN_VIIB] = makeUiButton("VIIb", BTN_VIIB, bw, bh);
        uiButtons[BTN_IV] = makeUiButton("IV", BTN_IV, bw, bh);
        uiButtons[BTN_I] = makeUiButton("I", BTN_I, bw, bh);
        uiButtons[BTN_V] = makeUiButton("V", BTN_V, bw, bh);

        // Row 2: ii iv iii vii°
        uiButtons[BTN_II] = makeUiButton("ii", BTN_II, bw, bh);
        uiButtons[BTN_IV_MINOR] = makeUiButton("iv", BTN_IV_MINOR, bw, bh);
        uiButtons[BTN_III] = makeUiButton("iii", BTN_III, bw, bh);
        uiButtons[BTN_VII_DIM] = makeUiButton("vii\u00B0", BTN_VII_DIM, bw, bh);

        // Row 3: Maj7 No3 Sus4 M/m
        uiButtons[BTN_MAJ7] = makeUiButton("Maj7", BTN_MAJ7, bw, bh);
        uiButtons[BTN_NO3] = makeUiButton("No3", BTN_NO3, bw, bh);
        uiButtons[BTN_SUS4] = makeUiButton("Sus4", BTN_SUS4, bw, bh);
        uiButtons[BTN_MM] = makeUiButton("M/m", BTN_MM, bw, bh);

        // Row 4: Add2 Add7 Hept [blank]
        uiButtons[BTN_ADD2] = makeUiButton("Add2", BTN_ADD2, bw, bh);
        uiButtons[BTN_ADD7] = makeUiButton("Add7", BTN_ADD7, bw, bh);
        uiButtons[BTN_HEPT] = makeUiButton("Hept", BTN_HEPT, bw, bh);
        Button blank = makeUiButton("", -1, bw, bh);
        blank.setEnabled(false);
        blank.setText("");
        blank.setBackgroundColor(0xFF000000);

        // Add in row-major order.
        grid.addView(uiButtons[BTN_VIIB]);
        grid.addView(uiButtons[BTN_IV]);
        grid.addView(uiButtons[BTN_I]);
        grid.addView(uiButtons[BTN_V]);

        grid.addView(uiButtons[BTN_II]);
        grid.addView(uiButtons[BTN_IV_MINOR]);
        grid.addView(uiButtons[BTN_III]);
        grid.addView(uiButtons[BTN_VII_DIM]);

        grid.addView(uiButtons[BTN_MAJ7]);
        grid.addView(uiButtons[BTN_NO3]);
        grid.addView(uiButtons[BTN_SUS4]);
        grid.addView(uiButtons[BTN_MM]);

        grid.addView(uiButtons[BTN_ADD2]);
        grid.addView(uiButtons[BTN_ADD7]);
        grid.addView(uiButtons[BTN_HEPT]);
        grid.addView(blank);

        root.addView(grid);

        setContentView(root);
        iv.requestFocus();
        updateUiButtons();

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
            updateUiButtons();
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
