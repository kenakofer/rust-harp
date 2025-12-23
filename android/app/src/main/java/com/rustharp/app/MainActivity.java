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
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.FrameLayout;
import android.widget.GridLayout;
import android.widget.ImageButton;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.Spinner;
import android.widget.ArrayAdapter;
import android.content.SharedPreferences;

import java.util.ArrayList;
import java.util.List;

public class MainActivity extends Activity {

    // Spinner uses the 12 chromatic options, but names flat keys with flats.
    private static final String[] KEY_SPINNER_LABELS = new String[]{"C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"};

    // Note naming depends on key signature preference.
    private static final String[] NOTE_NAMES_SHARP = new String[]{"C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"};
    private static final String[] NOTE_NAMES_FLAT  = new String[]{"C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"};

    // Prefer flats in flat keys: Db, Eb, Ab, Bb.
    private static boolean preferFlatsForKey(int keyPc) {
        int k = ((keyPc % 12) + 12) % 12;
        return k == 1 || k == 3 || k == 5 || k == 8 || k == 10;
    }
    private static final int BTN_VIIB = 0;

    // Used to keep gesture-exclusion rects centered near the active touch.
    private int lastTouchY = -1;
    private static final int BTN_IV = 1;
    private static final int BTN_I = 2;
    private static final int BTN_V = 3;
    private static final int BTN_II = 4;
    private static final int BTN_VI = 5;
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

    private boolean showNoteNames = false;
    private boolean playOnTap = true;
    private boolean showRomanChords = true;
    private int keyIndex = 0;
    private Spinner keySpinner;
    private boolean updatingKeySpinner = false;
    private SharedPreferences prefs;

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

    public static native void rustSetShowNoteNames(long handle, boolean show);
    public static native void rustSetPlayOnTap(long handle, boolean enabled);
    public static native int rustSetKeyIndex(long handle, int keyIndex);
    public static native int rustGetKeyIndex(long handle);

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

        if (keySpinner != null) {
            int idx = rustGetKeyIndex(rustHandle);
            if (idx != keyIndex) {
                keyIndex = idx;
                updatingKeySpinner = true;
                keySpinner.setSelection(keyIndex);
                updatingKeySpinner = false;
                updateChordButtonLabels();
            }
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

    private Button makeBlank(int wPx, int hPx) {
        Button b = makeUiButton("", -1, wPx, hPx);
        // Keep layout spacing, but don't draw or intercept touches.
        b.setEnabled(false);
        b.setVisibility(View.INVISIBLE);
        return b;
    }

    private void updateChordButtonLabels() {
        // These must match the spinner’s key labels.
        int k = ((keyIndex % 12) + 12) % 12;
        String[] keys = preferFlatsForKey(k) ? NOTE_NAMES_FLAT : NOTE_NAMES_SHARP;

        if (showRomanChords) {
            if (uiButtons[BTN_VIIB] != null) uiButtons[BTN_VIIB].setText("VIIb");
            if (uiButtons[BTN_IV] != null) uiButtons[BTN_IV].setText("IV");
            if (uiButtons[BTN_I] != null) uiButtons[BTN_I].setText("I");
            if (uiButtons[BTN_V] != null) uiButtons[BTN_V].setText("V");
            if (uiButtons[BTN_II] != null) uiButtons[BTN_II].setText("ii");
            if (uiButtons[BTN_VI] != null) uiButtons[BTN_VI].setText("vi");
            if (uiButtons[BTN_III] != null) uiButtons[BTN_III].setText("iii");
            if (uiButtons[BTN_VII_DIM] != null) uiButtons[BTN_VII_DIM].setText("vii\u00B0");
            return;
        }

        // Scale degrees (in semitones) relative to key root.
        int viib = (k + 10) % 12; // bVII
        int iv = (k + 5) % 12;
        int i = k;
        int v = (k + 7) % 12;
        int ii = (k + 2) % 12;
        int vi = (k + 9) % 12;
        int iii = (k + 4) % 12;
        int viiDim = (k + 11) % 12;

        if (uiButtons[BTN_VIIB] != null) uiButtons[BTN_VIIB].setText(keys[viib]);
        if (uiButtons[BTN_IV] != null) uiButtons[BTN_IV].setText(keys[iv]);
        if (uiButtons[BTN_I] != null) uiButtons[BTN_I].setText(keys[i]);
        if (uiButtons[BTN_V] != null) uiButtons[BTN_V].setText(keys[v]);

        if (uiButtons[BTN_II] != null) uiButtons[BTN_II].setText(keys[ii] + "m");
        if (uiButtons[BTN_VI] != null) uiButtons[BTN_VI].setText(keys[vi] + "m");
        if (uiButtons[BTN_III] != null) uiButtons[BTN_III].setText(keys[iii] + "m");
        if (uiButtons[BTN_VII_DIM] != null) uiButtons[BTN_VII_DIM].setText(keys[viiDim] + "dim");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        rustInit(); // smoke test: ensures JNI wiring is alive
        rustHandle = rustCreateFrontend();

        prefs = getSharedPreferences("rustharp", MODE_PRIVATE);
        showNoteNames = prefs.getBoolean("showNoteNames", false);
        playOnTap = prefs.getBoolean("playOnTap", true);
        showRomanChords = prefs.getBoolean("showRomanChords", true);
        keyIndex = prefs.getInt("keyIndex", 0);
        rustSetShowNoteNames(rustHandle, showNoteNames);
        rustSetPlayOnTap(rustHandle, playOnTap);
        rustSetKeyIndex(rustHandle, keyIndex);

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
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q && e.getPointerCount() > 0) {
                    lastTouchY = (int) e.getY(0);
                    updateGestureExclusion();
                }
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
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    lastTouchY = (int) e.getY(idx);
                    updateGestureExclusion();
                }
            } else if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_POINTER_UP) {
                phase = 2;
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q && e.getPointerCount() <= 1) {
                    lastTouchY = -1;
                    updateGestureExclusion();
                }
            } else if (action == MotionEvent.ACTION_CANCEL) {
                phase = 3;
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    lastTouchY = -1;
                    updateGestureExclusion();
                }
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
        grid.setColumnCount(7);
        grid.setRowCount(3);
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

        // ~30% bigger than before.
        int bw = dpToPx(83);
        int bh = dpToPx(55);


        // Row 1: VIIb IV I V
        uiButtons[BTN_VIIB] = makeUiButton("VIIb", BTN_VIIB, bw, bh);
        uiButtons[BTN_IV] = makeUiButton("IV", BTN_IV, bw, bh);
        uiButtons[BTN_I] = makeUiButton("I", BTN_I, bw, bh);
        uiButtons[BTN_V] = makeUiButton("V", BTN_V, bw, bh);

        // Row 2: ii vi iii vii°
        uiButtons[BTN_II] = makeUiButton("ii", BTN_II, bw, bh);
        uiButtons[BTN_VI] = makeUiButton("vi", BTN_VI, bw, bh);
        uiButtons[BTN_III] = makeUiButton("iii", BTN_III, bw, bh);
        uiButtons[BTN_VII_DIM] = makeUiButton("vii\u00B0", BTN_VII_DIM, bw, bh);

        // Row 3: Maj7 No3 Sus4 M/m Add2 Add7 Hept
        uiButtons[BTN_MAJ7] = makeUiButton("Maj7", BTN_MAJ7, bw, bh);
        uiButtons[BTN_NO3] = makeUiButton("No3", BTN_NO3, bw, bh);
        uiButtons[BTN_SUS4] = makeUiButton("Sus4", BTN_SUS4, bw, bh);
        uiButtons[BTN_MM] = makeUiButton("M/m", BTN_MM, bw, bh);
        uiButtons[BTN_ADD2] = makeUiButton("Add2", BTN_ADD2, bw, bh);
        uiButtons[BTN_ADD7] = makeUiButton("Add7", BTN_ADD7, bw, bh);
        uiButtons[BTN_HEPT] = makeUiButton("Hept", BTN_HEPT, bw, bh);

        // Add in row-major order.
        grid.addView(uiButtons[BTN_VIIB]);
        grid.addView(uiButtons[BTN_IV]);
        grid.addView(uiButtons[BTN_I]);
        grid.addView(uiButtons[BTN_V]);
        grid.addView(makeBlank(bw, bh));
        grid.addView(makeBlank(bw, bh));
        grid.addView(makeBlank(bw, bh));

        grid.addView(uiButtons[BTN_II]);
        grid.addView(uiButtons[BTN_VI]);
        grid.addView(uiButtons[BTN_III]);
        grid.addView(uiButtons[BTN_VII_DIM]);
        grid.addView(makeBlank(bw, bh));
        grid.addView(makeBlank(bw, bh));
        grid.addView(makeBlank(bw, bh));

        grid.addView(uiButtons[BTN_MAJ7]);
        grid.addView(uiButtons[BTN_NO3]);
        grid.addView(uiButtons[BTN_SUS4]);
        grid.addView(uiButtons[BTN_MM]);
        grid.addView(uiButtons[BTN_ADD2]);
        grid.addView(uiButtons[BTN_ADD7]);
        grid.addView(uiButtons[BTN_HEPT]);

        root.addView(grid);

        // Status panel (lower-right): key selector.
        LinearLayout status = new LinearLayout(this);
        status.setOrientation(LinearLayout.VERTICAL);
        status.setBackgroundColor(0x88000000);
        status.setPadding(dpToPx(4), dpToPx(4), dpToPx(4), dpToPx(4));
        FrameLayout.LayoutParams slp = new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT);
        slp.gravity = android.view.Gravity.BOTTOM | android.view.Gravity.END;
        slp.rightMargin = 0;
        slp.bottomMargin = 0;
        status.setLayoutParams(slp);

        keySpinner = new Spinner(this);
        ArrayAdapter<String> adapter = new ArrayAdapter<>(this, android.R.layout.simple_spinner_item, KEY_SPINNER_LABELS);
        adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        keySpinner.setAdapter(adapter);
        keySpinner.setSelection(keyIndex);
        keySpinner.setOnItemSelectedListener(new android.widget.AdapterView.OnItemSelectedListener() {
            @Override
            public void onItemSelected(android.widget.AdapterView<?> parent, View view, int position, long id) {
                if (rustHandle == 0 || updatingKeySpinner) return;
                keyIndex = position;
                if (prefs != null) {
                    prefs.edit().putInt("keyIndex", keyIndex).apply();
                }
                updateChordButtonLabels();
                int flags = rustSetKeyIndex(rustHandle, keyIndex);
                if ((flags & 1) != 0) redraw();
                if (flags != 0) updateUiButtons();
            }

            @Override
            public void onNothingSelected(android.widget.AdapterView<?> parent) {
            }
        });

        status.addView(keySpinner);
        root.addView(status);

        // Options (upper-right): gear icon + simple popup.
        ImageButton gear = new ImageButton(this);
        gear.setImageResource(android.R.drawable.ic_menu_manage);
        gear.setBackgroundColor(0x00000000);
        int gearSize = dpToPx(40);
        FrameLayout.LayoutParams gearLp = new FrameLayout.LayoutParams(gearSize, gearSize);
        gearLp.gravity = android.view.Gravity.TOP | android.view.Gravity.END;
        gearLp.topMargin = dpToPx(4);
        gearLp.rightMargin = dpToPx(4);
        gear.setLayoutParams(gearLp);

        LinearLayout options = new LinearLayout(this);
        options.setOrientation(LinearLayout.VERTICAL);
        options.setBackgroundColor(0xCC000000);
        options.setPadding(dpToPx(8), dpToPx(8), dpToPx(8), dpToPx(8));
        FrameLayout.LayoutParams optLp = new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT);
        optLp.gravity = android.view.Gravity.TOP | android.view.Gravity.END;
        optLp.topMargin = dpToPx(48);
        optLp.rightMargin = dpToPx(4);
        options.setLayoutParams(optLp);
        options.setVisibility(android.view.View.GONE);

        CheckBox cb = new CheckBox(this);
        cb.setText("Note names");
        cb.setTextColor(0xFFFFFFFF);
        cb.setChecked(showNoteNames);
        cb.setOnCheckedChangeListener((buttonView, isChecked) -> {
            showNoteNames = isChecked;
            if (prefs != null) {
                prefs.edit().putBoolean("showNoteNames", showNoteNames).apply();
            }
            if (rustHandle != 0) {
                rustSetShowNoteNames(rustHandle, showNoteNames);
            }
            redraw();
        });
        options.addView(cb);

        CheckBox cbTap = new CheckBox(this);
        cbTap.setText("Play on tap");
        cbTap.setTextColor(0xFFFFFFFF);
        cbTap.setChecked(playOnTap);
        cbTap.setOnCheckedChangeListener((buttonView, isChecked) -> {
            playOnTap = isChecked;
            if (prefs != null) {
                prefs.edit().putBoolean("playOnTap", playOnTap).apply();
            }
            if (rustHandle != 0) {
                rustSetPlayOnTap(rustHandle, playOnTap);
            }
        });
        options.addView(cbTap);

        CheckBox cbRoman = new CheckBox(this);
        cbRoman.setText("Roman chords");
        cbRoman.setTextColor(0xFFFFFFFF);
        cbRoman.setChecked(showRomanChords);
        cbRoman.setOnCheckedChangeListener((buttonView, isChecked) -> {
            showRomanChords = isChecked;
            if (prefs != null) {
                prefs.edit().putBoolean("showRomanChords", showRomanChords).apply();
            }
            updateChordButtonLabels();
        });
        options.addView(cbRoman);

        gear.setOnClickListener(v -> {
            options.setVisibility(options.getVisibility() == android.view.View.VISIBLE
                    ? android.view.View.GONE
                    : android.view.View.VISIBLE);
        });

        root.addView(options);
        root.addView(gear);

        setContentView(root);
        iv.requestFocus();
        updateChordButtonLabels();
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

        // Android limits the gesture-exclusion size (notably in height along the edge).
        // Keep it as a band near the active touch (or centered if idle).
        int bandHeight = dpToPx(200);
        int cy = (lastTouchY >= 0) ? lastTouchY : (height / 2);
        int top = Math.max(0, cy - (bandHeight / 2));
        int bottom = Math.min(height, top + bandHeight);

        List<Rect> rects = new ArrayList<>();
        // Due to bevel, It needs to extend off the left edge somewhat:
        rects.add(new Rect(-edgePx, top, leftEdge, bottom));
        // And off the right edge as well:
        rects.add(new Rect(rightEdgeStart, top, width + edgePx, bottom));
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
    public void onBackPressed() {
        // Don't allow system back gesture/button to exit the app while playing.
        // (We still allow the user to leave via the launcher/task switcher.)
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
