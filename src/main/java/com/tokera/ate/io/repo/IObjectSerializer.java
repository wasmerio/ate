package com.tokera.ate.io.repo;

import com.tokera.ate.dao.base.BaseDao;
import org.checkerframework.checker.nullness.qual.NonNull;

public interface IObjectSerializer {

    byte[] serializeObj(@NonNull BaseDao obj);

    <T extends BaseDao> T deserializeObj(byte[] bytes, Class<T> clazz);
}
