package com.rustharp.app.gesture;

import java.util.ArrayList;
import java.util.List;

/**
 * Distance-agnostic cardinal gesture recognizer.
 *
 * - Commit a segment once movement crosses gestureDistancePx along a cardinal direction.
 * - After each commit, advance a virtual anchor by exactly gestureDistancePx.
 * - The previous (forward) direction is blocked for the next segment, and overshoot in that
 *   direction is clamped so BACK does not require retracing overshoot.
 */
public final class GestureRecognizer {
    private final float d;

    private boolean active = false;

    private float downX;
    private float downY;

    private float anchorX;
    private float anchorY;

    // Last committed absolute direction; also the blocked forward direction for the next segment.
    private Dir lastDir = null;

    private final List<Dir> committedAbs = new ArrayList<>();
    private final List<Turn> turns = new ArrayList<>();

    public GestureRecognizer(float gestureDistancePx) {
        this.d = gestureDistancePx;
    }

    public void onDown(float x, float y) {
        active = true;
        downX = x;
        downY = y;
        anchorX = x;
        anchorY = y;
        lastDir = null;
        committedAbs.clear();
        turns.clear();
    }

    public GestureDebugState debugState() {
        return new GestureDebugState(
                active,
                d,
                downX,
                downY,
                anchorX,
                anchorY,
                lastDir,
                committedAbs,
                turns
        );
    }

    public void onMove(float x, float y) {
        if (!active) return;

        // A single move event can legitimately commit multiple segments.
        while (true) {
            // Distance-agnostic behavior: once a direction is committed, further motion in that
            // blocked (forward) direction drags the virtual anchor along with the finger.
            if (lastDir != null) {
                float dx0 = x - anchorX;
                float dy0 = y - anchorY;
                float forward = lastDir.proj(dx0, dy0);
                if (forward > 0) {
                    switch (lastDir) {
                        case LEFT:
                            anchorX -= forward;
                            break;
                        case RIGHT:
                            anchorX += forward;
                            break;
                        case UP:
                            anchorY -= forward;
                            break;
                        case DOWN:
                            anchorY += forward;
                            break;
                    }
                }
            }

            float dx = x - anchorX;
            float dy = y - anchorY;

            Dir commit;

            if (lastDir == null) {
                float ax = Math.abs(dx);
                float ay = Math.abs(dy);
                if (ax < d && ay < d) return;

                // Deterministic dominant-axis commit; ties prefer horizontal.
                if (ax >= ay) {
                    commit = dx >= 0 ? Dir.RIGHT : Dir.LEFT;
                } else {
                    commit = dy >= 0 ? Dir.DOWN : Dir.UP;
                }
            } else {
                Dir ccw = lastDir.ccw();
                Dir cw = lastDir.cw();
                Dir back = lastDir.opposite();

                float pCcw = ccw.proj(dx, dy);
                float pCw = cw.proj(dx, dy);
                float pBack = back.proj(dx, dy);

                commit = null;
                float best = 0;

                if (pCcw >= d && pCcw > best) {
                    best = pCcw;
                    commit = ccw;
                }
                if (pCw >= d && pCw > best) {
                    best = pCw;
                    commit = cw;
                }
                if (pBack >= d && pBack > best) {
                    best = pBack;
                    commit = back;
                }

                if (commit == null) return;
            }

            commitDir(commit);
            // Continue loop: there might be enough remaining displacement for another commit.
        }
    }

    public GestureResult onUp(float x, float y) {
        if (!active) {
            return new GestureResult(null, turns, committedAbs);
        }
        active = false;
        Dir initial = committedAbs.isEmpty() ? null : committedAbs.get(0);
        return new GestureResult(initial, turns, committedAbs);
    }

    public void onCancel() {
        active = false;
        lastDir = null;
        committedAbs.clear();
        turns.clear();
    }

    private void commitDir(Dir dir) {
        if (lastDir != null) {
            Turn t = Turn.fromDirs(lastDir, dir);
            if (t == null) {
                // Should be impossible: we only allow CCW/CW/BACK.
                return;
            }
            turns.add(t);
        }

        committedAbs.add(dir);
        lastDir = dir;

        // Advance anchor by exactly d.
        switch (dir) {
            case LEFT:
                anchorX -= d;
                break;
            case RIGHT:
                anchorX += d;
                break;
            case UP:
                anchorY -= d;
                break;
            case DOWN:
                anchorY += d;
                break;
        }
    }
}
