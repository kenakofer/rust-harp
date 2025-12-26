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
import android.widget.SeekBar;
import android.widget.TextView;
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
    private boolean showChordButtons = true;
    private int keyIndex = 0;
    private Spinner keySpinner;
    private boolean updatingKeySpinner = false;

    private String audioBackend = "AAudio"; // "AAudio" or "AudioTrack"
    private Spinner audioSpinner;
    private boolean updatingAudioSpinner = false;

    private int a4TuningHz = 440; // 430..450

    private SharedPreferences prefs;

    private GridLayout chordGrid;

    public static native int rustInit();
    public static native long rustCreateFrontend();
    public static native void rustDestroyFrontend(long handle);
    public static native int rustHandleAndroidKey(long handle, int keyCode, int unicodeChar, boolean isDown);
    public static native int rustHandleUiButton(long handle, int buttonId, boolean isDown);
    public static native int rustGetUiButtonsMask(long handle);
    public static native int rustHandleTouch(long handle, long pointerId, int phase, int x, int y, int width, int height, float pressure);

    public static native void rustSetAudioSampleRate(long handle, int sampleRateHz);
    public static native int rustFillAudio(long handle, int frames, short[] outPcm);

    public static native boolean rustStartAAudio(long handle);
    public static native void rustStopAAudio(long handle);
    public static native void rustResetAudioChannel(long handle);

    public static native void rustSetShowNoteNames(long handle, boolean show);
    public static native void rustSetPlayOnTap(long handle, boolean enabled);
    public static native void rustSetA4TuningHz(long handle, int a4TuningHz);
    public static native int rustSetKeyIndex(long handle, int keyIndex);
    public static native int rustGetKeyIndex(long handle);

    // App-only chord-wheel behavior knobs.
    public static native void rustSetImpliedSevenths(long handle, boolean enabled);
    public static native void rustSetChordReleaseNoteOffDelayMs(long handle, int ms);
    public static native void rustFlushDeferredNoteOffs(long handle);
    public static native int rustApplyChordWheelChoice(long handle, int chordButtonId, int dir8);
    public static native int rustToggleChordWheelMinorMajor(long handle, int chordButtonId);

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

    // ---- Chord wheel gesture (app-only) ----
    private static final int WHEEL_NONE = -1;
    private static final int WHEEL_DEADZONE_DP = 10;
    private static final int DOUBLE_TAP_MS = 350;

    private int wheelActiveButton = -1;
    private int wheelPointerId = -1;
    private float wheelDownX = 0;
    private float wheelDownY = 0;
    private int wheelDir = WHEEL_NONE;

    private int lastTapButton = -1;
    private long lastTapUpMs = 0;

    private WheelOverlayView wheelOverlay;

    private int dpToPxF(int dp) {
        return (int) (dp * getResources().getDisplayMetrics().density);
    }

    private int quantizeDir8(float dx, float dy) {
        // dx,dy in screen coords (dy positive down). Return 0..7 where 0=N and clockwise.
        double a = Math.atan2(dy, dx);          // 0=east
        double shifted = a + Math.PI / 2.0;    // 0=north
        double twoPi = Math.PI * 2.0;
        double norm = (shifted % twoPi + twoPi) % twoPi;
        double sector = (norm + (Math.PI / 8.0)) / (Math.PI / 4.0);
        return ((int) Math.floor(sector)) & 7;
    }

    private android.view.View.OnTouchListener chordWheelTouchListener(int chordBtnId) {
        return (v, e) -> {
            if (rustHandle == 0) return true;
            int action = e.getActionMasked();

            if (action == MotionEvent.ACTION_DOWN) {
                // Only allow one wheel at a time (simpler + avoids multi-chord ambiguity).
                if (wheelActiveButton != -1) return true;

                wheelActiveButton = chordBtnId;
                wheelPointerId = e.getPointerId(0);
                wheelDownX = e.getX();
                wheelDownY = e.getY();
                wheelDir = WHEEL_NONE;

                // Clear any prior wheel modifiers for a fresh triad selection.
                // This also re-applies the chord immediately (while held).
                rustApplyChordWheelChoice(rustHandle, chordBtnId, -1);

                if (wheelOverlay != null) {
                    boolean isMajorDegree = (chordBtnId == BTN_VIIB || chordBtnId == BTN_IV || chordBtnId == BTN_I || chordBtnId == BTN_V);
                    wheelOverlay.setWheelState((Button) v, isMajorDegree, wheelDir);
                }
                redraw();
                updateUiButtons();
                return true;
            }

            if (action == MotionEvent.ACTION_MOVE && wheelActiveButton == chordBtnId) {
                int idx = e.findPointerIndex(wheelPointerId);
                if (idx < 0) return true;
                float x = e.getX(idx);
                float y = e.getY(idx);
                float dx = x - wheelDownX;
                float dy = y - wheelDownY;
                float dist2 = dx * dx + dy * dy;
                int dead = dpToPxF(WHEEL_DEADZONE_DP);
                if (dist2 >= dead * dead) {
                    int dir = quantizeDir8(dx, dy);
                    if (dir != wheelDir) {
                        wheelDir = dir;
                        int flags = rustApplyChordWheelChoice(rustHandle, chordBtnId, dir);
                        if ((flags & 1) != 0) redraw();
                        updateUiButtons();
                        if (wheelOverlay != null) {
                            boolean isMajorDegree = (chordBtnId == BTN_VIIB || chordBtnId == BTN_IV || chordBtnId == BTN_I || chordBtnId == BTN_V);
                            wheelOverlay.setWheelState((Button) v, isMajorDegree, wheelDir);
                        }
                    }
                }
                return true;
            }

            if ((action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL) && wheelActiveButton == chordBtnId) {
                long now = android.os.SystemClock.uptimeMillis();
                boolean wasTap = (wheelDir == WHEEL_NONE);

                // End wheel state.
                wheelActiveButton = -1;
                wheelPointerId = -1;
                wheelDir = WHEEL_NONE;
                if (wheelOverlay != null) {
                    wheelOverlay.clearWheel();
                }

                if (action == MotionEvent.ACTION_UP && wasTap) {
                    if (lastTapButton == chordBtnId && (now - lastTapUpMs) <= DOUBLE_TAP_MS) {
                        // Toggle while the chord is still logically held, then release below.
                        int flags = rustToggleChordWheelMinorMajor(rustHandle, chordBtnId);
                        if ((flags & 1) != 0) redraw();
                        updateUiButtons();
                        lastTapButton = -1;
                        lastTapUpMs = 0;
                    } else {
                        // Arm double-tap window.
                        lastTapButton = chordBtnId;
                        lastTapUpMs = now;

                        // Tiny visual cue: briefly show what a double-tap would toggle to.
                        if (v instanceof Button) {
                            Button b = (Button) v;
                            String old = b.getText().toString();
                            String min = WheelOverlayView.minorOf(old);
                            String maj = WheelOverlayView.majorOf(old);
                            String cue = old.equals(min) ? maj : min;
                            b.setText(cue);
                            b.postDelayed(() -> {
                                // Only restore if it hasn't been overwritten.
                                if (cue.contentEquals(b.getText())) {
                                    b.setText(old);
                                }
                            }, DOUBLE_TAP_MS);
                        }
                    }
                }

                // Always release at the end (double-tap toggle simulates a press in Rust).
                rustHandleUiButton(rustHandle, chordBtnId, false);

                // Flush deferred note-offs once the double-tap window has expired.
                v.postDelayed(() -> {
                    if (rustHandle != 0) {
                        rustFlushDeferredNoteOffs(rustHandle);
                    }
                }, DOUBLE_TAP_MS);

                redraw();
                updateUiButtons();
                return true;
            }

            return true;
        };
    }

    private static class WheelOverlayView extends android.view.View {
        private Button anchor;
        private int dir = WHEEL_NONE;
        private boolean anchorIsMajorDegree = true;

        private final android.graphics.Paint pFill = new android.graphics.Paint();
        private final android.graphics.Paint pStroke = new android.graphics.Paint();
        private final android.graphics.Paint pText = new android.graphics.Paint();

        WheelOverlayView(android.content.Context ctx) {
            super(ctx);
            setWillNotDraw(false);
            pFill.setAntiAlias(true);
            pStroke.setAntiAlias(true);
            pStroke.setStyle(android.graphics.Paint.Style.STROKE);
            pStroke.setStrokeWidth(2);
            pStroke.setColor(0x66FFFFFF);

            pText.setAntiAlias(true);
            pText.setColor(0xFFFFFFFF);
            pText.setTextAlign(android.graphics.Paint.Align.CENTER);
        }

        void setWheelState(Button anchor, boolean anchorIsMajorDegree, int dir) {
            this.anchor = anchor;
            this.anchorIsMajorDegree = anchorIsMajorDegree;
            this.dir = dir;
            setVisibility(android.view.View.VISIBLE);
            bringToFront();
            invalidate();
        }

        void clearWheel() {
            this.anchor = null;
            this.dir = WHEEL_NONE;
            setVisibility(android.view.View.GONE);
            invalidate();
        }

        private static boolean looksRoman(String s) {
            if (s.isEmpty()) return false;
            char c = s.charAt(0);
            return c == 'I' || c == 'V' || c == 'i' || c == 'v';
        }

        private static String romanCase(String s, boolean upper) {
            StringBuilder out = new StringBuilder(s.length());
            for (int i = 0; i < s.length(); i++) {
                char c = s.charAt(i);
                if (c == 'I' || c == 'V' || c == 'i' || c == 'v') {
                    out.append(upper ? Character.toUpperCase(c) : Character.toLowerCase(c));
                } else {
                    out.append(c);
                }
            }
            return out.toString();
        }

        private static String minorOf(String base) {
            // Roman: lowercase (keep flats/other suffixes). Absolute: add 'm' unless already minor/dim.
            if (base.endsWith("dim") || base.endsWith("m")) return base;
            if (looksRoman(base)) return romanCase(base, false);
            return base + "m";
        }

        private static String majorOf(String base) {
            // Roman: uppercase (keep flats/other suffixes). Absolute: strip trailing 'm'.
            if (base.endsWith("m")) return base.substring(0, base.length() - 1);
            if (looksRoman(base)) return romanCase(base, true);
            return base;
        }

        private static String[] labelsFor(String base, boolean majorDegree) {
            // Caret-notation matches the spec (IV^7, iv^M7, etc) and renders reliably.
            String maj = base;
            String min = minorOf(base);
            String majFromMin = majorOf(base);

            if (majorDegree) {
                return new String[]{
                        maj + "^7",
                        maj + "^9",
                        maj + "+2",
                        min + "^9",
                        min + "^7",
                        min + "^M7",
                        maj + "sus",
                        maj + "^M7",
                };
            }

            return new String[]{
                    majFromMin + "^7",
                    majFromMin + "^9",
                    base + "+2",
                    base + "^9",
                    base + "^7",
                    base + "^M7",
                    base + "sus",
                    majFromMin + "^M7",
            };
        }

        @Override
        protected void onDraw(android.graphics.Canvas c) {
            super.onDraw(c);
            if (anchor == null) return;

            int[] loc = new int[2];
            int[] root = new int[2];
            anchor.getLocationOnScreen(loc);
            getLocationOnScreen(root);

            float left = loc[0] - root[0];
            float top = loc[1] - root[1];
            float cx = left + anchor.getWidth() / 2.0f;
            float cy = top + anchor.getHeight() / 2.0f;
            float r = Math.max(anchor.getWidth(), anchor.getHeight()) * 0.95f;

            android.graphics.RectF oval = new android.graphics.RectF(cx - r, cy - r, cx + r, cy + r);

            // Draw wedges (8 sectors). Align sector centers with the quantizer (N is straight up).
            final float startDeg = -112.5f; // -90 - 22.5
            for (int i = 0; i < 8; i++) {
                boolean sel = (i == dir);
                pFill.setStyle(android.graphics.Paint.Style.FILL);
                pFill.setColor(sel ? 0x66FFFFFF : 0x22000000);
                c.drawArc(oval, startDeg + i * 45, 45, true, pFill);
                c.drawArc(oval, startDeg + i * 45, 45, true, pStroke);
            }

            // Labels.
            String base = anchor.getText().toString();
            String[] labels = labelsFor(base, anchorIsMajorDegree);
            pText.setTextSize(Math.max(18.0f, r * 0.18f));

            float labelR = r * 0.72f;
            for (int i = 0; i < 8; i++) {
                float midDeg = -90 + i * 45;
                double rad = Math.toRadians(midDeg);
                float tx = cx + (float) (Math.cos(rad) * labelR);
                float ty = cy + (float) (Math.sin(rad) * labelR);

                boolean sel = (i == dir);
                pText.setColor(sel ? 0xFF000000 : 0xFFFFFFFF);
                // Center vertically on the baseline a bit.
                c.drawText(labels[i], tx, ty + pText.getTextSize() * 0.35f, pText);
            }
        }
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
        showChordButtons = prefs.getBoolean("showChordButtons", true);
        keyIndex = prefs.getInt("keyIndex", 0);
        audioBackend = prefs.getString("audioBackend", "AAudio");
        rustSetShowNoteNames(rustHandle, showNoteNames);
        rustSetPlayOnTap(rustHandle, playOnTap);
        rustSetKeyIndex(rustHandle, keyIndex);
        // App chord buttons should not auto-generate implied sevenths when multi-pressed.
        rustSetImpliedSevenths(rustHandle, false);
        // Defer chord-change note-offs until chord release + double-tap window.
        rustSetChordReleaseNoteOffDelayMs(rustHandle, DOUBLE_TAP_MS);

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
                    float p = e.getPressure(i);
                    int flags = rustHandleTouch(rustHandle, e.getPointerId(i), 1, (int) e.getX(i), (int) e.getY(i), w, h, p);
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

            float p = e.getPressure(idx);
            float size = e.getSize(idx);
            float major = e.getToolMajor(idx);
            Log.d("RustHarp", "touch " + (phase==0?"down":(phase==2?"up":"other")) + " p=" + p + " size=" + size + " major=" + major);
            int flags = rustHandleTouch(rustHandle, pid, phase, (int) e.getX(idx), (int) e.getY(idx), w, h, p);
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

        // Touch chord grid (lower-left). Modifier buttons removed; modifiers are selected via a swipe-wheel.
        chordGrid = new GridLayout(this);
        chordGrid.setColumnCount(4);
        chordGrid.setRowCount(2);
        chordGrid.setUseDefaultMargins(false);
        chordGrid.setPadding(0, 0, 0, 0);
        chordGrid.setMotionEventSplittingEnabled(true);

        FrameLayout.LayoutParams glp = new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT);
        glp.leftMargin = dpToPx(30);
        glp.bottomMargin = dpToPx(30);
        glp.gravity = android.view.Gravity.BOTTOM | android.view.Gravity.START;
        chordGrid.setLayoutParams(glp);
        chordGrid.setVisibility(showChordButtons ? View.VISIBLE : View.GONE);

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

        // Chord-wheel gesture replaces the default button press logic for these chord buttons.
        uiButtons[BTN_VIIB].setOnTouchListener(chordWheelTouchListener(BTN_VIIB));
        uiButtons[BTN_IV].setOnTouchListener(chordWheelTouchListener(BTN_IV));
        uiButtons[BTN_I].setOnTouchListener(chordWheelTouchListener(BTN_I));
        uiButtons[BTN_V].setOnTouchListener(chordWheelTouchListener(BTN_V));
        uiButtons[BTN_II].setOnTouchListener(chordWheelTouchListener(BTN_II));
        uiButtons[BTN_VI].setOnTouchListener(chordWheelTouchListener(BTN_VI));
        uiButtons[BTN_III].setOnTouchListener(chordWheelTouchListener(BTN_III));
        uiButtons[BTN_VII_DIM].setOnTouchListener(chordWheelTouchListener(BTN_VII_DIM));

        // Add in row-major order.
        chordGrid.addView(uiButtons[BTN_VIIB]);
        chordGrid.addView(uiButtons[BTN_IV]);
        chordGrid.addView(uiButtons[BTN_I]);
        chordGrid.addView(uiButtons[BTN_V]);

        chordGrid.addView(uiButtons[BTN_II]);
        chordGrid.addView(uiButtons[BTN_VI]);
        chordGrid.addView(uiButtons[BTN_III]);
        chordGrid.addView(uiButtons[BTN_VII_DIM]);

        root.addView(chordGrid);

        // Wheel overlay draws above the chord buttons (made visible only while a wheel gesture is active).
        wheelOverlay = new WheelOverlayView(this);
        wheelOverlay.setLayoutParams(new FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT));
        wheelOverlay.setVisibility(View.GONE);
        root.addView(wheelOverlay);

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

        CheckBox cbButtons = new CheckBox(this);
        cbButtons.setText("Chord buttons");
        cbButtons.setTextColor(0xFFFFFFFF);
        cbButtons.setChecked(showChordButtons);
        cbButtons.setOnCheckedChangeListener((buttonView, isChecked) -> {
            showChordButtons = isChecked;
            if (prefs != null) {
                prefs.edit().putBoolean("showChordButtons", showChordButtons).apply();
            }
            if (chordGrid != null) {
                chordGrid.setVisibility(showChordButtons ? View.VISIBLE : View.GONE);
            }
        });
        options.addView(cbButtons);

        // Audio backend selection (applies on restart for now).
        audioSpinner = new Spinner(this);
        String[] audioLabels = new String[]{"AAudio", "AudioTrack"};
        ArrayAdapter<String> audioAdapter = new ArrayAdapter<>(this, android.R.layout.simple_spinner_item, audioLabels);
        audioAdapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        audioSpinner.setAdapter(audioAdapter);
        audioSpinner.setSelection("AudioTrack".equals(audioBackend) ? 1 : 0);
        audioSpinner.setOnItemSelectedListener(new android.widget.AdapterView.OnItemSelectedListener() {
            @Override
            public void onItemSelected(android.widget.AdapterView<?> parent, View view, int position, long id) {
                if (updatingAudioSpinner) return;
                String chosen = (position == 1) ? "AudioTrack" : "AAudio";
                if (!chosen.equals(audioBackend)) {
                    audioBackend = chosen;
                    if (prefs != null) {
                        prefs.edit().putString("audioBackend", audioBackend).apply();
                    }
                    applyAudioBackend();
                }
            }

            @Override
            public void onNothingSelected(android.widget.AdapterView<?> parent) {
            }
        });
        options.addView(audioSpinner);

        // Tuning: A4 reference in Hz (430..450)
        TextView tuningLabel = new TextView(this);
        tuningLabel.setTextColor(0xFFFFFFFF);
        tuningLabel.setText("A4 " + a4TuningHz + "Hz");
        options.addView(tuningLabel);

        SeekBar tuning = new SeekBar(this);
        tuning.setMax(20);
        tuning.setProgress(Math.max(0, Math.min(20, a4TuningHz - 430)));
        tuning.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
            @Override
            public void onProgressChanged(SeekBar seekBar, int progress, boolean fromUser) {
                int hz = 430 + progress;
                if (hz == a4TuningHz) return;
                a4TuningHz = hz;
                tuningLabel.setText("A4 " + a4TuningHz + "Hz");
                if (prefs != null) {
                    prefs.edit().putInt("a4TuningHz", a4TuningHz).apply();
                }
                if (rustHandle != 0) {
                    rustSetA4TuningHz(rustHandle, a4TuningHz);
                }
            }

            @Override
            public void onStartTrackingTouch(SeekBar seekBar) {
            }

            @Override
            public void onStopTrackingTouch(SeekBar seekBar) {
            }
        });
        options.addView(tuning);

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

        // Start selected audio backend.
        applyAudioBackend();
    }

    private void applyAudioBackend() {
        if (rustHandle == 0) return;

        // Stop anything currently running.
        if (audio != null) {
            audio.stop();
            audio = null;
        }
        rustStopAAudio(rustHandle);

        // Recreate the Rust audio message channel so AAudio/AudioTrack can both attach cleanly.
        rustResetAudioChannel(rustHandle);
        rustSetA4TuningHz(rustHandle, a4TuningHz);

        if ("AudioTrack".equals(audioBackend)) {
            audio = new RustAudio(rustHandle, (android.media.AudioManager) getSystemService(AUDIO_SERVICE));
            audio.start();
            return;
        }

        boolean aaudioOk = rustStartAAudio(rustHandle);
        if (!aaudioOk) {
            Log.w("RustHarp", "AAudio failed; falling back to AudioTrack");
            updatingAudioSpinner = true;
            audioBackend = "AudioTrack";
            if (audioSpinner != null) audioSpinner.setSelection(1);
            updatingAudioSpinner = false;
            if (prefs != null) prefs.edit().putString("audioBackend", audioBackend).apply();

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
