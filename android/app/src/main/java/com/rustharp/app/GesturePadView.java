package com.rustharp.app;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Typeface;
import android.util.Log;
import android.view.MotionEvent;
import android.view.View;

import com.rustharp.app.gesture.GestureDebugState;

import com.rustharp.app.gesture.Dir;
import com.rustharp.app.gesture.GestureChordMapper;
import com.rustharp.app.gesture.GestureRecognizer;
import com.rustharp.app.gesture.GestureResult;
import com.rustharp.app.gesture.Turn;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

/** Gesture pad with chord labels showing available/pending chords. */
public final class GesturePadView extends View {
    private static void drawDirArm(Canvas c, float x, float y, Dir dir, float d, Paint p) {
        float nx = x;
        float ny = y;
        switch (dir) {
            case LEFT: nx -= d; break;
            case RIGHT: nx += d; break;
            case UP: ny -= d; break;
            case DOWN: ny += d; break;
        }
        c.drawLine(x, y, nx, ny, p);
    }

    private static void drawChevron(Canvas c, float cx, float cy, Dir dir, float radius, Paint p) {
        // Draw a small outward-pointing chevron (>) just outside the circle edge.
        float angle = 0;
        switch (dir) {
            case UP:    angle = (float) Math.toRadians(-90); break;
            case DOWN:  angle = (float) Math.toRadians(90); break;
            case LEFT:  angle = (float) Math.toRadians(180); break;
            case RIGHT: angle = (float) Math.toRadians(0); break;
        }

        // Position the tip outside the circle
        float chevronSize = 24.0f; // 2x the original 12px
        float tipDistance = radius + chevronSize * 0.3f; // Move tip outside by 30% of chevron size
        float tipX = cx + tipDistance * (float) Math.cos(angle);
        float tipY = cy + tipDistance * (float) Math.sin(angle);

        float chevronAngle = (float) Math.toRadians(30); // 30° spread

        // Left arm of chevron
        float lx = tipX - chevronSize * (float) Math.cos(angle - chevronAngle);
        float ly = tipY - chevronSize * (float) Math.sin(angle - chevronAngle);
        c.drawLine(tipX, tipY, lx, ly, p);

        // Right arm of chevron
        float rx = tipX - chevronSize * (float) Math.cos(angle + chevronAngle);
        float ry = tipY - chevronSize * (float) Math.sin(angle + chevronAngle);
        c.drawLine(tipX, tipY, rx, ry, p);
    }

    public interface Listener {
        void onChordGestureCommitted(GesturePadView pad, Dir initialDir, int modifiersMask);
    }

    private static final float GESTURE_DISTANCE_PX = 70.0f;
    private static final float VIS_SCALE = GESTURE_DISTANCE_PX / 30.0f;

    private final GestureRecognizer gr = new GestureRecognizer(GESTURE_DISTANCE_PX);

    private final Paint pBg = new Paint();
    private final Paint pPath = new Paint();
    private final Paint pAnchor = new Paint();
    private final Paint pAllowed = new Paint();
    private final Paint pBlocked = new Paint();
    private final Paint pFinger = new Paint();
    private final Paint pChordLabel = new Paint();
    private final Paint pChordCenter = new Paint();
    private final Paint pCircleOutline = new Paint();
    private final Paint pChevron = new Paint();

    private float lastX = 0;
    private float lastY = 0;
    private boolean showOverlay = false;

    private int activePointerId = -1;
    private Listener listener;
    private boolean minorPad = false;

    private boolean useRomanChords = true;
    private int keyPc = 0;

    private List<Turn> pendingTurns = Collections.emptyList();
    private boolean gestureActive = false;  // Track if gesture is in progress

    public GesturePadView(Context ctx) {
        super(ctx);
        setClickable(true);

        pBg.setColor(0x22FFFFFF);

        pPath.setColor(0xFFFFFFFF);
        pPath.setStrokeWidth(4.0f * VIS_SCALE);
        pPath.setStyle(Paint.Style.STROKE);
        pPath.setAntiAlias(true);

        pAnchor.setColor(0xFFFFFF00);
        pAnchor.setStyle(Paint.Style.FILL);
        pAnchor.setAntiAlias(true);

        pAllowed.setColor(0x6600FF00);
        pAllowed.setStrokeWidth(10.0f * VIS_SCALE);
        pAllowed.setStyle(Paint.Style.STROKE);
        pAllowed.setAntiAlias(true);

        pBlocked.setColor(0x66FF0000);
        pBlocked.setStrokeWidth(10.0f * VIS_SCALE);
        pBlocked.setStyle(Paint.Style.STROKE);
        pBlocked.setAntiAlias(true);

        pFinger.setColor(0xAA00FFFF);
        pFinger.setStyle(Paint.Style.FILL);
        pFinger.setAntiAlias(true);

        pChordLabel.setColor(0xFFFFFFFF);
        pChordLabel.setTextSize(36.0f);  // 2x the original 18
        pChordLabel.setTextAlign(Paint.Align.CENTER);
        pChordLabel.setAntiAlias(true);
        pChordLabel.setTypeface(Typeface.SERIF);

        pChordCenter.setColor(0xFFFFFF00);
        pChordCenter.setTextSize(28.0f);
        pChordCenter.setTextAlign(Paint.Align.CENTER);
        pChordCenter.setAntiAlias(true);
        pChordCenter.setFakeBoldText(true);
        pChordCenter.setTypeface(Typeface.SERIF);

        pCircleOutline.setColor(0xFFFFFFFF);
        pCircleOutline.setStyle(Paint.Style.STROKE);
        pCircleOutline.setStrokeWidth(2.0f);
        pCircleOutline.setAntiAlias(true);

        pChevron.setColor(0xFFFFFFFF);
        pChevron.setStyle(Paint.Style.STROKE);
        pChevron.setStrokeWidth(3.0f);
        pChevron.setStrokeCap(Paint.Cap.ROUND);
        pChevron.setAntiAlias(true);
    }

    public void setListener(Listener l) {
        this.listener = l;
    }

    public void setMinorPad(boolean enabled) {
        this.minorPad = enabled;
    }

    public void setChordPreferences(boolean useRoman, int keyPitchClass) {
        this.useRomanChords = useRoman;
        this.keyPc = keyPitchClass;
        invalidate();
    }

    private int rootDegreeForDir(Dir dir) {
        if (minorPad) {
            switch (dir) {
                case UP:    return 9;  // vi
                case LEFT:  return 2;  // ii
                case RIGHT: return 4;  // iii
                case DOWN:  return 11; // vii°
            }
        } else {
            switch (dir) {
                case UP:    return 0;  // I
                case LEFT:  return 5;  // IV
                case RIGHT: return 7;  // V
                case DOWN:  return 10; // bVII
            }
        }
        return 0;
    }

    @Override
    protected void onDraw(Canvas c) {
        super.onDraw(c);
        c.drawRect(0, 0, getWidth(), getHeight(), pBg);

        float w = getWidth();
        float h = getHeight();
        float cx = w / 2;
        float cy = h / 2;
        float baseSize = Math.min(w, h);
        float scale = 1.0f;

        // During active gesture, center on finger position and scale up 40%.
        if (gestureActive) {
            cx = lastX;
            cy = lastY;
            scale = 1.4f;
        }

        float radius = baseSize * 0.45f * scale;

        // Draw circle outline around the edge labels (follows finger during gesture).
        c.drawCircle(cx, cy, radius, pCircleOutline);

        // Determine available next directions and the root chord based on gesture state.
        GestureDebugState st = gr.debugState();
        List<Dir> availableDirs;
        Dir initialDir = null;

        // Check if we have an active gesture with committed directions.
        if (gestureActive && !st.committedAbsDirs.isEmpty()) {
            initialDir = st.committedAbsDirs.get(0);
        }

        if (initialDir == null) {
            // No gesture active: show initial 4 directions.
            availableDirs = Arrays.asList(Dir.UP, Dir.LEFT, Dir.RIGHT, Dir.DOWN);
        } else {
            // In-progress gesture: show CCW/CW/BACK relative to last direction.
            Dir lastDir = st.lastDir;
            if (lastDir != null) {
                availableDirs = Arrays.asList(lastDir.ccw(), lastDir.cw(), lastDir.opposite());
            } else {
                availableDirs = Arrays.asList(Dir.UP, Dir.LEFT, Dir.RIGHT, Dir.DOWN);
            }
        }

        // Draw chord labels around the edge for each available direction.
        for (Dir dir : availableDirs) {
            float labelX = cx;
            float labelY = cy;
            float offset = baseSize * 0.4f * scale;

            switch (dir) {
                case UP:    labelY -= offset; break;
                case DOWN:  labelY += offset; break;
                case LEFT:  labelX -= offset; break;
                case RIGHT: labelX += offset; break;
            }

            List<Turn> nextTurns;
            int rootDeg;

            if (initialDir == null) {
                // No gesture yet: show the chord for starting this direction with no turns.
                nextTurns = Collections.emptyList();
                rootDeg = rootDegreeForDir(dir);
            } else {
                // In-progress gesture: compute next turn from current state, keep initial root.
                nextTurns = new ArrayList<>(pendingTurns);
                Turn nextTurn = com.rustharp.app.gesture.Turn.fromDirs(st.lastDir, dir);
                if (nextTurn != null) {
                    nextTurns.add(nextTurn);
                }
                rootDeg = rootDegreeForDir(initialDir);  // Always use initial root
            }

            String chordName = ChordNamer.formatChord(rootDeg, nextTurns, minorPad, useRomanChords, keyPc);
            c.drawText(chordName, labelX, labelY + pChordLabel.getTextSize() / 3, pChordLabel);
        }

        // Draw pending chord in static pad center (show as soon as we have an initial direction).
        if (initialDir != null) {
            float padCx = w / 2;
            float padCy = h / 2;
            int rootDeg = rootDegreeForDir(initialDir);
            String centerChord = ChordNamer.formatChord(rootDeg, pendingTurns, minorPad, useRomanChords, keyPc);
            c.drawText(centerChord, padCx, padCy + pChordCenter.getTextSize() / 3, pChordCenter);
        }

        if (!showOverlay) return;

        // Debug overlay visuals.
        float d = st.gestureDistancePx;

        // Draw committed path (virtual), including any anchor dragging in the blocked direction.
        float endX = st.downX;
        float endY = st.downY;
        for (int i = 0; i < st.committedAbsDirs.size(); i++) {
            Dir dir = st.committedAbsDirs.get(i);
            switch (dir) {
                case LEFT: endX -= d; break;
                case RIGHT: endX += d; break;
                case UP: endY -= d; break;
                case DOWN: endY += d; break;
            }
        }
        float offX = st.anchorX - endX;
        float offY = st.anchorY - endY;

        float x = st.downX + offX;
        float y = st.downY + offY;
        for (int i = 0; i < st.committedAbsDirs.size(); i++) {
            Dir dir = st.committedAbsDirs.get(i);
            float nx = x;
            float ny = y;
            switch (dir) {
                case LEFT: nx -= d; break;
                case RIGHT: nx += d; break;
                case UP: ny -= d; break;
                case DOWN: ny += d; break;
            }
            c.drawLine(x, y, nx, ny, pPath);
            x = nx;
            y = ny;
        }

        // Current anchor + threshold radius.
        c.drawCircle(st.anchorX, st.anchorY, 8.0f * VIS_SCALE, pAnchor);
        c.drawCircle(st.anchorX, st.anchorY, d, pAllowed);

        // Allowed/blocked directions from the anchor.
        if (st.lastDir == null) {
            // Initial: show the 4 cardinal directions.
            c.drawLine(st.anchorX, st.anchorY, st.anchorX - d, st.anchorY, pAllowed);
            c.drawLine(st.anchorX, st.anchorY, st.anchorX + d, st.anchorY, pAllowed);
            c.drawLine(st.anchorX, st.anchorY, st.anchorX, st.anchorY - d, pAllowed);
            c.drawLine(st.anchorX, st.anchorY, st.anchorX, st.anchorY + d, pAllowed);
        } else {
            Dir ccw = st.lastDir.ccw();
            Dir cw = st.lastDir.cw();
            Dir back = st.lastDir.opposite();

            drawDirArm(c, st.anchorX, st.anchorY, ccw, d, pAllowed);
            drawDirArm(c, st.anchorX, st.anchorY, cw, d, pAllowed);
            drawDirArm(c, st.anchorX, st.anchorY, back, d, pAllowed);

            // Blocked forward direction.
            drawDirArm(c, st.anchorX, st.anchorY, st.lastDir, d, pBlocked);
        }

        // Finger position.
        c.drawCircle(lastX, lastY, 10.0f * VIS_SCALE, pFinger);
    }

    @Override
    public boolean onTouchEvent(MotionEvent e) {
        int action = e.getActionMasked();

        if (action == MotionEvent.ACTION_DOWN) {
            activePointerId = e.getPointerId(0);
            lastX = e.getX(0);
            lastY = e.getY(0);
            showOverlay = true;
            pendingTurns = Collections.emptyList();  // Clear on new gesture
            gestureActive = true;  // Mark gesture as active
            gr.onDown(lastX, lastY);
            invalidate();
            return true;
        }

        if (activePointerId < 0) return false;

        if (action == MotionEvent.ACTION_MOVE) {
            int pi = e.findPointerIndex(activePointerId);
            if (pi >= 0) {
                lastX = e.getX(pi);
                lastY = e.getY(pi);
                gr.onMove(lastX, lastY);
                
                // Update pending turns for visual feedback.
                GestureDebugState updated = gr.debugState();
                pendingTurns = new ArrayList<>(updated.turns);
                
                invalidate();
            }
            return true;
        }

        if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL) {
            int pi = e.findPointerIndex(activePointerId);
            float x = (pi >= 0) ? e.getX(pi) : 0;
            float y = (pi >= 0) ? e.getY(pi) : 0;
            lastX = x;
            lastY = y;
            GestureResult r = gr.onUp(x, y);
            activePointerId = -1;
            invalidate();

            if (r.initial == null) {
                showOverlay = false;
                pendingTurns = Collections.emptyList();
                gestureActive = false;  // Gesture ended
                invalidate();  // Reset to show basic 4 directions
                return true;
            }

            Integer mods = GestureChordMapper.modifiersForTurns(r.turns, minorPad);
            if (mods == null) {
                Log.d("RustHarp", "gesture initial=" + r.initial + " turns=" + r.turns + " => undefined");
                showOverlay = false;
                pendingTurns = Collections.emptyList();
                gestureActive = false;  // Gesture ended
                invalidate();  // Reset to show basic 4 directions
                return true;
            }

            Log.d("RustHarp", "gesture initial=" + r.initial + " turns=" + r.turns + " mods=0x" + Integer.toHexString(mods));

            showOverlay = false;
            pendingTurns = Collections.emptyList();
            gestureActive = false;  // Gesture ended
            invalidate();  // Reset to show basic 4 directions after commit

            if (listener != null) {
                listener.onChordGestureCommitted(this, r.initial, mods);
            }

            // Keep it visible briefly for debugging; cleared on next down.
            postDelayed(() -> {
                showOverlay = false;
                invalidate();
            }, 250);

            return true;
        }

        // Ignore multi-pointer inside the pad for now.
        return true;
    }
}
