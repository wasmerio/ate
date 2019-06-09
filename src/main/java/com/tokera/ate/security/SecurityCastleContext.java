package com.tokera.ate.security;

import java.util.UUID;

public class SecurityCastleContext {
    public final UUID id;
    public final byte[] key;

    public SecurityCastleContext(UUID id, byte[] key) {
        this.id = id;
        this.key = key;
    }
}
