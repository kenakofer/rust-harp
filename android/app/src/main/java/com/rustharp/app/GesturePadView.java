package com.rustharp.app;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
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

    private float lastX = 0;
    private float lastY = 0;
    private boolean showOverlay = false;

    private int activePointerId = -1;
    private Listener listener;
    private boolean minorPad = false;

    private boolean useRomanChords = true;
    private int keyPc = 0;

    private List<Turn> pendingTurns = Collections.emptyList();

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
        pChordLabel.setTextSize(18.0f);
        pChordLabel.setTextAlign(Paint.Align.CENTER);
        pChordLabel.setAntiAlias(true);

        pChordCenter.setColor(0xFFFFFF00);
        pChordCenter.setTextSize(28.0f);
        pChordCenter.setTextAlign(Paint.Align.CENTER);
        pChordCenter.setAntiAlias(true);
        pChordCenter.setFakeBoldText(true);
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

        // Determine available next directions based on gesture state.
        GestureDebugState st = gr.debugState();
        List<Dir> availableDirs;

        if (st.lastDir == null) {
            // No gesture yet: show initial directions.
            availableDirs = Arrays.asList(Dir.UP, Dir.LEFT, Dir.RIGHT, Dir.DOWN);
        } else {
            // In-progress gesture: show CCW/CW/BACK.
            availableDirs = Arrays.asList(st.lastDir.ccw(), st.lastDir.cw(), st.lastDir.opposite());
        }

        // Draw chord labels around the edge for each available direction.
        for (Dir dir : availableDirs) {
            float labelX = cx;
            float labelY = cy;
            float offset = Math.min(w, h) * 0.4f;

            switch (dir) {
                case UP:    labelY -= offset; break;
                case DOWN:  labelY += offset; break;
                case LEFT:  labelX -= offset; break;
                case RIGHT: labelX += offset; break;
            }

            List<Turn> nextTurns;
            if (st.lastDir == null) {
                // Initial direction => no turns yet.
                nextTurns = Collections.emptyList();
            } else {
                // Append the turn for this direction.
                nextTurns = new ArrayList<>(pendingTurns);
                Turn nextTurn = com.rustharp.app.gesture.Turn.fromDirs(st.lastDir, dir);
                if (nextTurn != null) {
                    nextTurns.add(nextTurn);
                }
            }

            int rootDeg = rootDegreeForDir(st.lastDir == null ? dir : availableDirs.get(0));
            String chordName = ChordNamer.formatChord(rootDeg, nextTurns, minorPad, useRomanChords, keyPc);
            c.drawText(chordName, labelX, labelY + pChordLabel.getTextSize() / 3, pChordLabel);
        }

        // Draw pending chord in center.
        if (!pendingTurns.isEmpty() && st.lastDir != null) {
            Dir initial = st.committedAbsDirs.isEmpty() ? null : st.committedAbsDirs.get(0);
            if (initial != null) {
                int rootDeg = rootDegreeForDir(initial);
                String centerChord = ChordNamer.formatChord(rootDeg, pendingTurns, minorPad, useRomanChords, keyPc);
                c.drawText(centerChord, cx, cy + pChordCenter.getTextSize() / 3, pChordCenter);
            }
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
                invalidate();
                return true;
            }

            Integer mods = GestureChordMapper.modifiersForTurns(r.turns, minorPad);
            if (mods == null) {
                Log.d("RustHarp", "gesture initial=" + r.initial + " turns=" + r.turns + " => undefined");
                showOverlay = false;
                pendingTurns = Collections.emptyList();
                invalidate();
                return true;
            }

            Log.d("RustHarp", "gesture initial=" + r.initial + " turns=" + r.turns + " mods=0x" + Integer.toHexString(mods));

            showOverlay = false;
            pendingTurns = Collections.emptyList();

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
