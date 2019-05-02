package com.tokera.ate.delegates;

import com.jsoniter.JsonIterator;
import com.jsoniter.output.JsonStream;
import com.jsoniter.spi.JsoniterSpi;
import com.jsoniter.spi.TypeLiteral;
import com.jsoniter.static_codegen.StaticCodegenConfig;
import com.tokera.ate.io.repo.IObjectSerializer;
import com.tokera.ate.scopes.Startup;
import org.checkerframework.checker.nullness.qual.NonNull;

import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import java.util.UUID;

@Startup
@ApplicationScoped
public class JsonObjectSerializerDelegate implements IObjectSerializer, StaticCodegenConfig {

    @PostConstruct
    public void init() {
        JsoniterSpi.registerTypeEncoder(UUID.class, (obj, stream) -> stream.writeVal(obj.toString()));
        JsoniterSpi.registerTypeDecoder(UUID.class, iter -> UUID.fromString(iter.readString()));
    }

    @Override
    public byte[] serializeObj(@NonNull Object obj) {
        return JsonStream.serialize(obj).getBytes();
    }

    @Override
    public <T> T deserializeObj(byte[] bytes, Class<T> clazz) {
        return JsonIterator.deserialize(bytes).as(clazz);
    }

    @Override
    public void setup() {
    }

    @Override
    public TypeLiteral[] whatToCodegen() {
        return AteDelegate.get().serializableObjectsExtension.asTypeLiterals();
    }
}
