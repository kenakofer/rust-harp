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
    public static final int MOD_MAJOR_TRI = 1 << 0;
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

    private static final Map<List<Turn>, Integer> TABLE_MAJOR;
    private static final Map<List<Turn>, Integer> TABLE_MINOR;

    static {
        Map<List<Turn>, Integer> maj = new HashMap<>();

        // [] => use Rust's default triad for the selected root.
        maj.put(Collections.emptyList(), 0);

        // CW family
        maj.put(Arrays.asList(Turn.CW), MOD_ADD_m7);
        maj.put(Arrays.asList(Turn.CW, Turn.CW), MOD_ADD_m7 | MOD_ADD_M2);
        maj.put(Arrays.asList(Turn.CW, Turn.BACK), MOD_ADD_M7);

        // CCW family
        maj.put(Arrays.asList(Turn.CCW), MOD_MINOR_TRI);
        maj.put(Arrays.asList(Turn.CCW, Turn.CW), MOD_DIMIN_TRI);
        maj.put(Arrays.asList(Turn.CCW, Turn.CW, Turn.CW), MOD_DIMIN_TRI | MOD_ADD_m7); // half-diminished 7
        maj.put(Arrays.asList(Turn.CCW, Turn.CW, Turn.CCW), MOD_DIMIN_TRI | MOD_ADD_M6); // fully diminished 7

        maj.put(Arrays.asList(Turn.CCW, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_m7);
        maj.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.CW), MOD_MINOR_TRI | MOD_ADD_M6);
        maj.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_m7 | MOD_ADD_M2);
        maj.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.BACK), MOD_MINOR_TRI | MOD_SUS4);

        maj.put(Arrays.asList(Turn.CCW, Turn.BACK), MOD_MINOR_TRI | MOD_ADD_M2);
        maj.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.CW), MOD_MINOR_TRI | MOD_ADD_M2 | MOD_ADD_M6);
        maj.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_M2 | MOD_ADD_m7);
        maj.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.BACK), MOD_MINOR_TRI | MOD_SUS4);

        // BACK family
        maj.put(Arrays.asList(Turn.BACK), MOD_ADD_M2);
        maj.put(Arrays.asList(Turn.BACK, Turn.CW), MOD_ADD_M6);
        maj.put(Arrays.asList(Turn.BACK, Turn.CCW), MOD_ADD_m7 | MOD_ADD_M2);
        maj.put(Arrays.asList(Turn.BACK, Turn.BACK), MOD_SUS4);

        TABLE_MAJOR = Collections.unmodifiableMap(maj);

        // Minor pad: same gestures, but with a few intentional differences.
        Map<List<Turn>, Integer> min = new HashMap<>();

        // [] => keep Rust's default triad for vi/ii/iii/vii°.
        min.put(Collections.emptyList(), 0);

        // CW family: force major triad so +m7 behaves like a dominant-style chord.
        min.put(Arrays.asList(Turn.CW), MOD_MAJOR_TRI | MOD_ADD_m7);
        min.put(Arrays.asList(Turn.CW, Turn.CW), MOD_MAJOR_TRI | MOD_ADD_m7 | MOD_ADD_M2);
        min.put(Arrays.asList(Turn.CW, Turn.BACK), MOD_MAJOR_TRI | MOD_ADD_M7);

        // CCW alone: "major-ify" (toggle) instead of "minor-ify".
        min.put(Arrays.asList(Turn.CCW), MOD_SWITCH_MINOR_MAJOR);

        // All other sequences currently match the major pad's mapping.
        min.put(Arrays.asList(Turn.CCW, Turn.CW), MOD_DIMIN_TRI);
        min.put(Arrays.asList(Turn.CCW, Turn.CW, Turn.CW), MOD_DIMIN_TRI | MOD_ADD_m7);
        min.put(Arrays.asList(Turn.CCW, Turn.CW, Turn.CCW), MOD_DIMIN_TRI | MOD_ADD_M6);

        min.put(Arrays.asList(Turn.CCW, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_m7);
        min.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.CW), MOD_MINOR_TRI | MOD_ADD_M6);
        min.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_m7 | MOD_ADD_M2);
        min.put(Arrays.asList(Turn.CCW, Turn.CCW, Turn.BACK), MOD_MINOR_TRI | MOD_SUS4);

        min.put(Arrays.asList(Turn.CCW, Turn.BACK), MOD_MINOR_TRI | MOD_ADD_M2);
        min.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.CW), MOD_MINOR_TRI | MOD_ADD_M2 | MOD_ADD_M6);
        min.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.CCW), MOD_MINOR_TRI | MOD_ADD_M2 | MOD_ADD_m7);
        min.put(Arrays.asList(Turn.CCW, Turn.BACK, Turn.BACK), MOD_MINOR_TRI | MOD_SUS4);

        min.put(Arrays.asList(Turn.BACK), MOD_ADD_M2);
        min.put(Arrays.asList(Turn.BACK, Turn.CW), MOD_ADD_M6);
        min.put(Arrays.asList(Turn.BACK, Turn.CCW), MOD_ADD_m7 | MOD_ADD_M2);
        min.put(Arrays.asList(Turn.BACK, Turn.BACK), MOD_SUS4);

        TABLE_MINOR = Collections.unmodifiableMap(min);
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
        return (minorPad ? TABLE_MINOR : TABLE_MAJOR).get(turns);
    }
}
