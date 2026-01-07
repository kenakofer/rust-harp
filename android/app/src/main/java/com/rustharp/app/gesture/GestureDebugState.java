package com.rustharp.app.gesture;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public final class GestureDebugState {
    public final boolean active;
    public final float gestureDistancePx;

    public final float downX;
    public final float downY;

    public final float anchorX;
    public final float anchorY;

    public final Dir lastDir;
    public final List<Dir> committedAbsDirs;
    public final List<Turn> turns;

    public GestureDebugState(
            boolean active,
            float gestureDistancePx,
            float downX,
            float downY,
            float anchorX,
            float anchorY,
            Dir lastDir,
            List<Dir> committedAbsDirs,
            List<Turn> turns
    ) {
        this.active = active;
        this.gestureDistancePx = gestureDistancePx;
        this.downX = downX;
        this.downY = downY;
        this.anchorX = anchorX;
        this.anchorY = anchorY;
        this.lastDir = lastDir;
        this.committedAbsDirs = Collections.unmodifiableList(new ArrayList<>(committedAbsDirs));
        this.turns = Collections.unmodifiableList(new ArrayList<>(turns));
    }
}
