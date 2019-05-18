package com.tokera.ate.delegates;

import com.jsoniter.JsonIterator;
import com.jsoniter.any.Any;
import com.jsoniter.output.JsonStream;
import com.jsoniter.spi.JsoniterSpi;
import com.jsoniter.spi.TypeLiteral;
import com.jsoniter.static_codegen.StaticCodegenConfig;
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
public class JsonObjectSerializerDelegate implements IObjectSerializer, StaticCodegenConfig {

    @SuppressWarnings({"known.nonnull", "argument.type.incompatible", "return.type.incompatible"})
    @PostConstruct
    public void init() {
        JsoniterSpi.registerTypeEncoder(UUID.class, (obj, stream) -> {
            String val = obj != null ? obj.toString() : null;
            stream.writeVal(val);
        });
        JsoniterSpi.registerTypeDecoder(UUID.class, iter -> {
            if (iter == null) return null;
            String val = iter.readString();
            if (val == null) return null;
            return UUID.fromString(val);
        });
        JsoniterSpi.registerTypeEncoder(PUUID.class, (obj, stream) -> {
            String val = obj != null ? obj.toString() : null;
            stream.writeVal(val);
        });
        JsoniterSpi.registerTypeDecoder(PUUID.class, iter -> {
            if (iter == null) return null;
            String val = iter.readString();
            if (val == null) return null;
            return PUUID.parse(val);
        });
    }

    @Override
    public byte[] serializeObj(@NonNull BaseDao obj) {
        return JsonStream.serialize(obj).getBytes();
    }

    @Override
    public <T extends BaseDao> T deserializeObj(byte[] bytes, Class<T> clazz) {
        Any ret = JsonIterator.deserialize(bytes);
        return ret.as(clazz);
    }

    @Override
    public void setup() {
    }

    @Override
    public TypeLiteral[] whatToCodegen() {
        return AteDelegate.get().serializableObjectsExtension.asTypeLiterals();
    }
}
