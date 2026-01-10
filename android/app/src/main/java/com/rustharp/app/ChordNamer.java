package com.rustharp.app;

import com.rustharp.app.gesture.Dir;
import com.rustharp.app.gesture.GestureChordMapper;
import com.rustharp.app.gesture.Turn;

import java.util.List;

/**
 * Helper to format chord names (Roman or absolute) for gesture pads.
 */
public final class ChordNamer {
    private static final String[] NOTE_NAMES_SHARP = new String[]{"C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"};
    private static final String[] NOTE_NAMES_FLAT  = new String[]{"C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"};

    private static boolean preferFlatsForKey(int keyPc) {
        int k = ((keyPc % 12) + 12) % 12;
        return k == 1 || k == 3 || k == 5 || k == 8 || k == 10;
    }

    /**
     * Format a chord name given root degree (semitones above key root), turns, and preferences.
     *
     * @param rootDegree  semitones above key root (0=I, 2=ii, 4=iii, 5=IV, 7=V, 9=vi, 10=bVII, 11=vii°)
     * @param turns       gesture turn sequence (used to infer quality)
     * @param minorPad    whether this is the minor pad (affects mapping)
     * @param useRoman    show Roman numerals vs absolute note names
     * @param keyPc       key pitch class (for absolute naming)
     */
    public static String formatChord(int rootDegree, List<Turn> turns, boolean minorPad, boolean useRoman, int keyPc) {
        Integer mods = GestureChordMapper.modifiersForTurns(turns, minorPad);
        if (mods == null) mods = 0;

        return formatChordWithMods(rootDegree, mods, useRoman, keyPc);
    }

    /**
     * Format a chord name given root degree and explicit modifier bits.
     */
    public static String formatChordWithMods(int rootDegree, int mods, boolean useRoman, int keyPc) {
        if (useRoman) {
            return formatRoman(rootDegree, mods, false);
        } else {
            int absolutePc = (keyPc + rootDegree) % 12;
            String[] keys = preferFlatsForKey(keyPc) ? NOTE_NAMES_FLAT : NOTE_NAMES_SHARP;
            return formatAbsolute(keys[absolutePc], rootDegree, mods);
        }
    }

    private static String formatRoman(int rootDegree, int mods, boolean minorPad) {
        // Determine base Roman numeral from root degree.
        String base;
        boolean isMinorDefault;
        boolean isDimDefault;

        switch (rootDegree) {
            case 0:  base = "I";    isMinorDefault = false; isDimDefault = false; break; // I
            case 2:  base = "ii";   isMinorDefault = true;  isDimDefault = false; break; // ii
            case 4:  base = "iii";  isMinorDefault = true;  isDimDefault = false; break; // iii
            case 5:  base = "IV";   isMinorDefault = false; isDimDefault = false; break; // IV
            case 7:  base = "V";    isMinorDefault = false; isDimDefault = false; break; // V
            case 9:  base = "vi";   isMinorDefault = true;  isDimDefault = false; break; // vi
            case 10: base = "VIIb"; isMinorDefault = false; isDimDefault = false; break; // bVII (major)
            case 11: base = "vii°"; isMinorDefault = false; isDimDefault = true;  break; // vii° (dim)
            default: base = "?";    isMinorDefault = false; isDimDefault = false; break;
        }

        // Apply modifiers to infer quality suffix.
        String suffix = "";

        boolean hasMajorTri = (mods & GestureChordMapper.MOD_MAJOR_TRI) != 0;
        boolean hasMinorTri = (mods & GestureChordMapper.MOD_MINOR_TRI) != 0;
        boolean hasDiminTri = (mods & GestureChordMapper.MOD_DIMIN_TRI) != 0;
        boolean hasAddm7    = (mods & GestureChordMapper.MOD_ADD_m7) != 0;
        boolean hasAddM7    = (mods & GestureChordMapper.MOD_ADD_M7) != 0;
        boolean hasAddM2    = (mods & GestureChordMapper.MOD_ADD_M2) != 0;
        boolean hasAddM6    = (mods & GestureChordMapper.MOD_ADD_M6) != 0;
        boolean hasSus4     = (mods & GestureChordMapper.MOD_SUS4) == GestureChordMapper.MOD_SUS4;
        boolean hasSwitch   = (mods & GestureChordMapper.MOD_SWITCH_MINOR_MAJOR) != 0;

        // Determine actual triad quality (after modifiers).
        boolean actuallyMinor = isMinorDefault;
        boolean actuallyDim = isDimDefault;

        if (hasMajorTri) { actuallyMinor = false; actuallyDim = false; }
        if (hasMinorTri) { actuallyMinor = true;  actuallyDim = false; }
        if (hasDiminTri) { actuallyMinor = false; actuallyDim = true;  }
        if (hasSwitch) {
            // Toggle logic
            if (actuallyMinor) { actuallyMinor = false; }
            else if (actuallyDim) { actuallyDim = false; actuallyMinor = false; }
            else { actuallyMinor = true; }
        }

        // Add 7th/extensions.
        if (hasAddm7 && hasAddM2) {
            suffix = "9";
        } else if (hasAddm7) {
            suffix = "7";
        } else if (hasAddM7) {
            suffix = "maj7";
        } else if (hasSus4) {
            suffix = "sus4";
        } else if (hasAddM2) {
            suffix = "add9";
        } else if (hasAddM6) {
            suffix = "6";
        }

        // Combine base + quality + suffix.
        if (actuallyDim && hasAddm7) {
            return base.replace("°", "ø") + suffix; // half-diminished
        } else if (actuallyDim) {
            return base + suffix;
        } else if (actuallyMinor && !isMinorDefault) {
            return base.toLowerCase() + suffix; // forced minor
        } else if (!actuallyMinor && isMinorDefault) {
            return base.toUpperCase() + suffix; // forced major
        } else {
            return base + suffix;
        }
    }

    private static String formatAbsolute(String rootNote, int rootDegree, int mods) {
        String suffix = "";

        boolean hasMajorTri = (mods & GestureChordMapper.MOD_MAJOR_TRI) != 0;
        boolean hasMinorTri = (mods & GestureChordMapper.MOD_MINOR_TRI) != 0;
        boolean hasDiminTri = (mods & GestureChordMapper.MOD_DIMIN_TRI) != 0;
        boolean hasAddm7    = (mods & GestureChordMapper.MOD_ADD_m7) != 0;
        boolean hasAddM7    = (mods & GestureChordMapper.MOD_ADD_M7) != 0;
        boolean hasAddM2    = (mods & GestureChordMapper.MOD_ADD_M2) != 0;
        boolean hasAddM6    = (mods & GestureChordMapper.MOD_ADD_M6) != 0;
        boolean hasSus4     = (mods & GestureChordMapper.MOD_SUS4) == GestureChordMapper.MOD_SUS4;
        boolean hasSwitch   = (mods & GestureChordMapper.MOD_SWITCH_MINOR_MAJOR) != 0;

        // Determine default quality based on root degree (same logic as Roman formatter).
        boolean isMinorDefault;
        boolean isDimDefault;

        switch (rootDegree) {
            case 0:  isMinorDefault = false; isDimDefault = false; break; // I
            case 2:  isMinorDefault = true;  isDimDefault = false; break; // ii
            case 4:  isMinorDefault = true;  isDimDefault = false; break; // iii
            case 5:  isMinorDefault = false; isDimDefault = false; break; // IV
            case 7:  isMinorDefault = false; isDimDefault = false; break; // V
            case 9:  isMinorDefault = true;  isDimDefault = false; break; // vi
            case 10: isMinorDefault = false; isDimDefault = false; break; // bVII
            case 11: isMinorDefault = false; isDimDefault = true;  break; // vii° (dim)
            default: isMinorDefault = false; isDimDefault = false; break;
        }

        boolean actuallyMinor = isMinorDefault;
        boolean actuallyDim = isDimDefault;

        // Apply triad overrides.
        if (hasMajorTri) { actuallyMinor = false; actuallyDim = false; }
        if (hasMinorTri) { actuallyMinor = true;  actuallyDim = false; }
        if (hasDiminTri) { actuallyMinor = false; actuallyDim = true; }
        if (hasSwitch) {
            // Toggle logic (same as Roman)
            if (actuallyMinor) { actuallyMinor = false; }
            else if (actuallyDim) { actuallyDim = false; actuallyMinor = false; }
            else { actuallyMinor = true; }
        }

        // Build quality + extensions.
        if (actuallyDim && hasAddm7 && hasAddM6) {
            suffix = "dim7";
        } else if (actuallyDim && hasAddm7) {
            suffix = "ø7";
        } else if (actuallyDim) {
            suffix = "dim";
        } else if (actuallyMinor) {
            if (hasAddm7 && hasAddM2) {
                suffix = "m9";
            } else if (hasAddm7) {
                suffix = "m7";
            } else if (hasAddM6) {
                suffix = "m6";
            } else {
                suffix = "m";
            }
        } else {
            if (hasAddm7 && hasAddM2) {
                suffix = "9";
            } else if (hasAddm7) {
                suffix = "7";
            } else if (hasAddM7) {
                suffix = "maj7";
            } else if (hasSus4) {
                suffix = "sus4";
            } else if (hasAddM2) {
                suffix = "add9";
            } else if (hasAddM6) {
                suffix = "6";
            }
        }

        return rootNote + suffix;
    }
}
