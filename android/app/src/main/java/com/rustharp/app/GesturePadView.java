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

/** A simple touch target that recognizes chord gestures; visuals come later. */
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

    private static final float GESTURE_DISTANCE_PX = 30.0f;

    private final GestureRecognizer gr = new GestureRecognizer(GESTURE_DISTANCE_PX);

    private final Paint pBg = new Paint();
    private final Paint pPath = new Paint();
    private final Paint pAnchor = new Paint();
    private final Paint pAllowed = new Paint();
    private final Paint pBlocked = new Paint();
    private final Paint pFinger = new Paint();

    private float lastX = 0;
    private float lastY = 0;
    private boolean showOverlay = false;

    private int activePointerId = -1;
    private Listener listener;

    public GesturePadView(Context ctx) {
        super(ctx);
        setClickable(true);

        pBg.setColor(0x22FFFFFF);

        pPath.setColor(0xFFFFFFFF);
        pPath.setStrokeWidth(4.0f);
        pPath.setStyle(Paint.Style.STROKE);
        pPath.setAntiAlias(true);

        pAnchor.setColor(0xFFFFFF00);
        pAnchor.setStyle(Paint.Style.FILL);
        pAnchor.setAntiAlias(true);

        pAllowed.setColor(0x6600FF00);
        pAllowed.setStrokeWidth(10.0f);
        pAllowed.setStyle(Paint.Style.STROKE);
        pAllowed.setAntiAlias(true);

        pBlocked.setColor(0x66FF0000);
        pBlocked.setStrokeWidth(10.0f);
        pBlocked.setStyle(Paint.Style.STROKE);
        pBlocked.setAntiAlias(true);

        pFinger.setColor(0xAA00FFFF);
        pFinger.setStyle(Paint.Style.FILL);
        pFinger.setAntiAlias(true);
    }

    public void setListener(Listener l) {
        this.listener = l;
    }

    @Override
    protected void onDraw(Canvas c) {
        super.onDraw(c);
        c.drawRect(0, 0, getWidth(), getHeight(), pBg);

        if (!showOverlay) return;

        GestureDebugState st = gr.debugState();
        float d = st.gestureDistancePx;

        // Draw committed path (virtual, distance-agnostic).
        float x = st.downX;
        float y = st.downY;
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
        c.drawCircle(st.anchorX, st.anchorY, 8.0f, pAnchor);
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
        c.drawCircle(lastX, lastY, 10.0f, pFinger);
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
                invalidate();
                return true;
            }

            Integer mods = GestureChordMapper.modifiersForTurns(r.turns);
            if (mods == null) {
                Log.d("RustHarp", "gesture initial=" + r.initial + " turns=" + r.turns + " => undefined");
                showOverlay = false;
                invalidate();
                return true;
            }

            Log.d("RustHarp", "gesture initial=" + r.initial + " turns=" + r.turns + " mods=0x" + Integer.toHexString(mods));

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
