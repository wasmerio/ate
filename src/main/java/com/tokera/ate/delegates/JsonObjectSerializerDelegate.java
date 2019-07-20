package com.tokera.ate.delegates;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.io.repo.IObjectSerializer;
import com.tokera.ate.scopes.Startup;
import org.checkerframework.checker.nullness.qual.NonNull;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import java.util.UUID;

@Startup
@ApplicationScoped
public class JsonObjectSerializerDelegate implements IObjectSerializer {
    AteDelegate d = AteDelegate.get();

    @Override
    public byte[] serializeObj(@NonNull BaseDao obj) {
        return d.json.serialize(obj).getBytes();
    }

    @Override
    public <T extends BaseDao> T deserializeObj(byte[] bytes, Class<T> clazz) {
        return (T)d.json.deserialize(new String(bytes), clazz);
    }
}
