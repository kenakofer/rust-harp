package com.rustharp.app.gesture;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public final class GestureResult {
    public final Dir initial;
    public final List<Turn> turns;
    public final List<Dir> committedAbsDirs;

    public GestureResult(Dir initial, List<Turn> turns, List<Dir> committedAbsDirs) {
        this.initial = initial;
        this.turns = Collections.unmodifiableList(new ArrayList<>(turns));
        this.committedAbsDirs = Collections.unmodifiableList(new ArrayList<>(committedAbsDirs));
    }
}
