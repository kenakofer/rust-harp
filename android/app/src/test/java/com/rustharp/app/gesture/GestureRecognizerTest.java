package com.rustharp.app.gesture;

import org.junit.Test;

import static org.junit.Assert.*;

import java.util.Arrays;

public class GestureRecognizerTest {
    private static final float D = 90.0f;

    @Test
    public void noCommitBelowThreshold() {
        GestureRecognizer gr = new GestureRecognizer(D);
        gr.onDown(0, 0);
        gr.onMove(0, D - 1);
        GestureResult r = gr.onUp(0, D - 1);
        assertNull(r.initial);
        assertTrue(r.turns.isEmpty());
        assertTrue(r.committedAbsDirs.isEmpty());
    }

    @Test
    public void initialCommitUp() {
        GestureRecognizer gr = new GestureRecognizer(D);
        gr.onDown(0, 0);
        gr.onMove(0, -D);
        GestureResult r = gr.onUp(0, -D);
        assertEquals(Dir.UP, r.initial);
        assertEquals(Arrays.asList(Dir.UP), r.committedAbsDirs);
        assertTrue(r.turns.isEmpty());
    }

    @Test
    public void initialCommitLeftWithDrift() {
        GestureRecognizer gr = new GestureRecognizer(D);
        gr.onDown(0, 0);
        gr.onMove(-D, D * 0.4f);
        GestureResult r = gr.onUp(-D, D * 0.4f);
        assertEquals(Dir.LEFT, r.initial);
    }

    @Test
    public void overshootDoesNotMakeBackHarder() {
        // Commit LEFT, overshoot far left, then move right only D should commit BACK.
        GestureRecognizer gr = new GestureRecognizer(D);
        gr.onDown(0, 0);
        gr.onMove(-100, 0); // commits LEFT at -90 anchor, but finger is at -100
        gr.onMove(0, 0);     // move right by 90 from committed anchor => BACK
        GestureResult r = gr.onUp(0, 0);

        assertEquals(Arrays.asList(Dir.LEFT, Dir.RIGHT), r.committedAbsDirs);
        assertEquals(Arrays.asList(Turn.BACK), r.turns);
    }

    @Test
    public void lateralDriftIsResetAtCommit() {
        // Commit UP with some horizontal drift; anchor should snap X to the finger position so CCW
        // (LEFT) still commits with exactly D additional motion.
        GestureRecognizer gr = new GestureRecognizer(D);
        gr.onDown(0, 0);
        gr.onMove(D * 0.2f, -D);          // UP (with drift)
        gr.onMove(D * 0.2f - D, -D);      // CCW from UP => LEFT
        GestureResult r = gr.onUp(D * 0.2f - D, -D);

        assertEquals(Arrays.asList(Dir.UP, Dir.LEFT), r.committedAbsDirs);
        assertEquals(Arrays.asList(Turn.CCW), r.turns);
    }

    @Test
    public void multiSegmentCommitUpThenCcwThenBack() {
        // UP then CCW (left) then BACK (right relative to left)
        GestureRecognizer gr = new GestureRecognizer(D);
        gr.onDown(0, 0);
        gr.onMove(0, -D);       // UP
        gr.onMove(-D, -D);      // CCW from UP => LEFT
        gr.onMove(-D, 0);       // BACK from LEFT => RIGHT
        GestureResult r = gr.onUp(-D, 0);

        assertEquals(Arrays.asList(Dir.UP, Dir.LEFT, Dir.RIGHT), r.committedAbsDirs);
        assertEquals(Arrays.asList(Turn.CCW, Turn.BACK), r.turns);
    }
}
