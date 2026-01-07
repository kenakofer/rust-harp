package com.rustharp.app;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.Log;
import android.view.MotionEvent;
import android.view.View;

import com.rustharp.app.gesture.Dir;
import com.rustharp.app.gesture.GestureChordMapper;
import com.rustharp.app.gesture.GestureRecognizer;
import com.rustharp.app.gesture.GestureResult;

/** A simple touch target that recognizes chord gestures; visuals come later. */
public final class GesturePadView extends View {
    public interface Listener {
        void onChordGestureCommitted(GesturePadView pad, Dir initialDir, int modifiersMask);
    }

    private static final float GESTURE_DISTANCE_PX = 30.0f;

    private final GestureRecognizer gr = new GestureRecognizer(GESTURE_DISTANCE_PX);
    private final Paint p = new Paint();

    private int activePointerId = -1;
    private Listener listener;

    public GesturePadView(Context ctx) {
        super(ctx);
        setClickable(true);
        p.setColor(0x22FFFFFF);
    }

    public void setListener(Listener l) {
        this.listener = l;
    }

    @Override
    protected void onDraw(Canvas c) {
        super.onDraw(c);
        // Temporary minimal pad hint.
        c.drawRect(0, 0, getWidth(), getHeight(), p);
    }

    @Override
    public boolean onTouchEvent(MotionEvent e) {
        int action = e.getActionMasked();

        if (action == MotionEvent.ACTION_DOWN) {
            activePointerId = e.getPointerId(0);
            gr.onDown(e.getX(0), e.getY(0));
            return true;
        }

        if (activePointerId < 0) return false;

        if (action == MotionEvent.ACTION_MOVE) {
            int pi = e.findPointerIndex(activePointerId);
            if (pi >= 0) {
                gr.onMove(e.getX(pi), e.getY(pi));
            }
            return true;
        }

        if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL) {
            int pi = e.findPointerIndex(activePointerId);
            float x = (pi >= 0) ? e.getX(pi) : 0;
            float y = (pi >= 0) ? e.getY(pi) : 0;
            GestureResult r = gr.onUp(x, y);
            activePointerId = -1;

            if (r.initial == null) return true;

            Integer mods = GestureChordMapper.modifiersForTurns(r.turns);
            if (mods == null) {
                Log.d("RustHarp", "gesture initial=" + r.initial + " turns=" + r.turns + " => undefined");
                return true;
            }

            Log.d("RustHarp", "gesture initial=" + r.initial + " turns=" + r.turns + " mods=0x" + Integer.toHexString(mods));

            if (listener != null) {
                listener.onChordGestureCommitted(this, r.initial, mods);
            }
            return true;
        }

        // Ignore multi-pointer inside the pad for now.
        return true;
    }
}
