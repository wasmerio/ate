package com.tokera.ate.io.repo;

import org.checkerframework.checker.nullness.qual.NonNull;

public interface IObjectSerializer {

    byte[] serializeObj(@NonNull Object obj);

    <T> T deserializeObj(byte[] bytes, Class<T> clazz);
}
