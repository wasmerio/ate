package com.tokera.ate.delegates;

/**
 * Delegate that reduces the amount of boiler plate injecting plus reduces the
 * amount of redirection over delegates and initialization steps for requests
 */
public final class AteDelegate extends BaseAteDelegate {
    protected static AteDelegate g_instance = new AteDelegate();

    public static AteDelegate get() {
        return g_instance;
    }

    public AteDelegate() {
    }
}