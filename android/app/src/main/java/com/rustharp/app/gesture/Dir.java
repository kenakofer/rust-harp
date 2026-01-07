package com.rustharp.app.gesture;

public enum Dir {
    UP,
    DOWN,
    LEFT,
    RIGHT;

    public Dir opposite() {
        switch (this) {
            case UP: return DOWN;
            case DOWN: return UP;
            case LEFT: return RIGHT;
            case RIGHT: return LEFT;
        }
        throw new IllegalStateException("unreachable");
    }

    /** CCW = turn 90° left relative to prior motion. */
    public Dir ccw() {
        switch (this) {
            case UP: return LEFT;
            case LEFT: return DOWN;
            case DOWN: return RIGHT;
            case RIGHT: return UP;
        }
        throw new IllegalStateException("unreachable");
    }

    /** CW = turn 90° right relative to prior motion. */
    public Dir cw() {
        switch (this) {
            case UP: return RIGHT;
            case RIGHT: return DOWN;
            case DOWN: return LEFT;
            case LEFT: return UP;
        }
        throw new IllegalStateException("unreachable");
    }

    float proj(float dx, float dy) {
        // Projection along this dir. Positive means displacement in that direction.
        switch (this) {
            case LEFT: return -dx;
            case RIGHT: return dx;
            case UP: return -dy;
            case DOWN: return dy;
        }
        throw new IllegalStateException("unreachable");
    }
}
