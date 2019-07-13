package com.tokera.ate.security.core;

import java.security.SecureRandom;

public interface IRandomFactory {
    
    SecureRandom getRandom();

    boolean idempotent();
}
