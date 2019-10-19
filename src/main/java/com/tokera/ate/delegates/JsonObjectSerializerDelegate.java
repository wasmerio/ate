package com.tokera.ate.delegates;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.smile.SmileFactory;
import com.fasterxml.jackson.module.afterburner.AfterburnerModule;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.io.repo.IObjectSerializer;
import com.tokera.ate.scopes.Startup;
import org.checkerframework.checker.nullness.qual.NonNull;

import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.WebApplicationException;
import java.io.IOException;

@Startup
@ApplicationScoped
public class JsonObjectSerializerDelegate implements IObjectSerializer {
    AteDelegate d = AteDelegate.get();
    private final ThreadLocal<ObjectMapper> mappers;

    public JsonObjectSerializerDelegate() {
        mappers = ThreadLocal.withInitial(() ->
        {
            ObjectMapper mapper = new ObjectMapper(new SmileFactory());
            mapper.configure(DeserializationFeature.FAIL_ON_MISSING_CREATOR_PROPERTIES, false);
            mapper.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
            mapper.configure(DeserializationFeature.FAIL_ON_IGNORED_PROPERTIES, false);
            mapper.configure(DeserializationFeature.READ_UNKNOWN_ENUM_VALUES_USING_DEFAULT_VALUE, true);
            mapper.registerModule(new AfterburnerModule());
            return mapper;
        });
    }

    @Override
    public byte[] serializeObj(@NonNull BaseDao obj) {
        ObjectMapper mapper = mappers.get();
        try {
            return mapper.writeValueAsBytes(obj);
        } catch (JsonProcessingException e) {
            throw new WebApplicationException("Failed to serialize object.", e);
        }
    }

    @Override
    @SuppressWarnings("unchecked")
    public <T extends BaseDao> T deserializeObj(byte[] bytes, Class<T> clazz) {
        ObjectMapper mapper = mappers.get();
        try {
            return mapper.readValue(bytes, clazz);
        } catch (IOException e) {
            throw new WebApplicationException("Failed to serialize object.", e);
        }
    }
}
