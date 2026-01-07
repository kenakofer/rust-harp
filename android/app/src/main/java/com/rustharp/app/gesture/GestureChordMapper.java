package com.rustharp.app.gesture;

import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Maps a gesture's turn sequence to a Rust Modifiers bitmask.
 *
 * Undefined sequences return null (no chord change).
 */
public final class GestureChordMapper {
    // Rust: src/chord.rs (bitflags Modifiers: u16)
    public static final int MOD_MINOR_TRI = 1 << 1;
    public static final int MOD_DIMIN_TRI = 1 << 2;
    public static final int MOD_ADD_M2 = 1 << 3;
    public static final int MOD_ADD_M6 = 1 << 4;
    public static final int MOD_ADD_m7 = 1 << 6;
    public static final int MOD_ADD_M7 = 1 << 7;
    public static final int MOD_SWITCH_MINOR_MAJOR = 1 << 10;
    public static final int MOD_ADD_4 = 1 << 11;
    public static final int MOD_NO3 = 1 << 12;

    public static final int MOD_SUS4 = MOD_ADD_4 | MOD_NO3;

    private static final Map<List<Turn>, Integer> TABLE;

    static {
        Map<List<Turn>, Integer> m = new HashMap<>();

        // [] => major triad (no forced mods)
        m.put(Collections.emptyList(), 0);

        // CW family
        m.put(Arrays.asList(Turn.CW), MOD_ADD_m7);
        m.put(Arrays.asList(Turn.CW, Turn.CW), MOD_ADD_m7 | MOD_ADD_M2);
        m.put(Arrays.asList(Turn.CW, Turn.BACK), MOD_ADD_M7);

        // CCW family
        m.put(Arrays.asList(Turn.CCW), MOD_MINOR_TRI);
        m.put(Arrays.asList(Turn.CCW, Turn.CW), MOD_DIMIN_TRI);
        m.put(Arrays.asList(Turn.CCW, Turn.CW, Turn.CW), MOD_DIMIN_TRI | MOD_ADD_m7); // half-diminished 7
        m.put(Arrays.asList(Turn.CCW, Turn.CW, Turn.CCW), MOD_DIMIN_TRI | MOD_ADD_M6); // fully diminished 7

        m.put(Arrays.asList(Turn.CCW, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_m7);
        m.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.CW), MOD_MINOR_TRI | MOD_ADD_M6);
        m.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_m7 | MOD_ADD_M2);
        m.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.BACK), MOD_MINOR_TRI | MOD_SUS4);

        m.put(Arrays.asList(Turn.CCW, Turn.BACK), MOD_MINOR_TRI | MOD_ADD_M2);
        m.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.CW), MOD_MINOR_TRI | MOD_ADD_M2 | MOD_ADD_M6);
        m.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_M2 | MOD_ADD_m7);
        m.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.BACK), MOD_MINOR_TRI | MOD_SUS4);

        // BACK family
        m.put(Arrays.asList(Turn.BACK), MOD_ADD_M2);
        m.put(Arrays.asList(Turn.BACK, Turn.CW), MOD_ADD_M6);
        m.put(Arrays.asList(Turn.BACK, Turn.CCW), MOD_ADD_m7 | MOD_ADD_M2);
        m.put(Arrays.asList(Turn.BACK, Turn.BACK), MOD_SUS4);

        TABLE = Collections.unmodifiableMap(m);
    }

    private GestureChordMapper() {
    }

    public static Integer modifiersForTurns(List<Turn> turns) {
        return modifiersForTurns(turns, false);
    }

    /**
     * @param minorPad if true, CCW alone "major-ifies" (toggle) instead of "minor-ifying".
     */
    public static Integer modifiersForTurns(List<Turn> turns, boolean minorPad) {
        if (minorPad && turns.size() == 1 && turns.get(0) == Turn.CCW) {
            return MOD_SWITCH_MINOR_MAJOR;
        }
        return TABLE.get(turns);
    }
}
