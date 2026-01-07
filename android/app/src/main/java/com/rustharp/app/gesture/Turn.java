package com.rustharp.app.gesture;

public enum Turn {
    CCW,
    CW,
    BACK;

    public static Turn fromDirs(Dir prev, Dir next) {
        if (next == prev.opposite()) return BACK;
        if (next == prev.ccw()) return CCW;
        if (next == prev.cw()) return CW;
        return null;
    }
}
