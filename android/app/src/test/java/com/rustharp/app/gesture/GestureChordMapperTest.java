package com.rustharp.app.gesture;

import org.junit.Test;

import static org.junit.Assert.*;

import java.util.Arrays;
import java.util.Collections;

public class GestureChordMapperTest {

    @Test
    public void emptyTurnsIsMajorTriadNoMods() {
        assertEquals(Integer.valueOf(0), GestureChordMapper.modifiersForTurns(Collections.emptyList()));
    }

    @Test
    public void basicMappings() {
        assertEquals(Integer.valueOf(GestureChordMapper.MOD_ADD_m7),
                GestureChordMapper.modifiersForTurns(Arrays.asList(Turn.CW)));
        assertEquals(Integer.valueOf(GestureChordMapper.MOD_MINOR_TRI),
                GestureChordMapper.modifiersForTurns(Arrays.asList(Turn.CCW)));
        assertEquals(Integer.valueOf(GestureChordMapper.MOD_ADD_M2),
                GestureChordMapper.modifiersForTurns(Arrays.asList(Turn.BACK)));
    }

    @Test
    public void minorPadCcwMajorifies() {
        assertEquals(Integer.valueOf(GestureChordMapper.MOD_SWITCH_MINOR_MAJOR),
                GestureChordMapper.modifiersForTurns(Arrays.asList(Turn.CCW), true));
    }

    @Test
    public void diminishedSevenths() {
        assertEquals(Integer.valueOf(GestureChordMapper.MOD_DIMIN_TRI | GestureChordMapper.MOD_ADD_m7),
                GestureChordMapper.modifiersForTurns(Arrays.asList(Turn.CCW, Turn.CW, Turn.CW)));
        assertEquals(Integer.valueOf(GestureChordMapper.MOD_DIMIN_TRI | GestureChordMapper.MOD_ADD_M6),
                GestureChordMapper.modifiersForTurns(Arrays.asList(Turn.CCW, Turn.CW, Turn.CCW)));
    }

    @Test
    public void undefinedReturnsNull() {
        assertNull(GestureChordMapper.modifiersForTurns(Arrays.asList(Turn.CW, Turn.CCW)));
    }
}
