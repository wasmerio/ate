package com.tokera.ate.delegates;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.tokera.ate.scopes.Startup;

import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.WebApplicationException;
import java.io.IOException;

@ApplicationScoped
@Startup
public class JsonDelegate {
    AteDelegate d = AteDelegate.get();
    private final ThreadLocal<ObjectMapper> mappers;

    public JsonDelegate() {
        mappers = ThreadLocal.withInitial(() ->
        {
            ObjectMapper mapper = new ObjectMapper();
            return mapper;
        });
    }

    public String serialize(Object obj) {
        ObjectMapper mapper = mappers.get();
        try {
            return mapper.writeValueAsString(obj);
        } catch (JsonProcessingException e) {
            throw new WebApplicationException("Failed to serialize object.", e);
        }
    }

    public <T> Object deserialize(String data, Class<T> clazz) {
        ObjectMapper mapper = mappers.get();
        try {
            return mapper.readValue(data, clazz);
        } catch (IOException e) {
            throw new WebApplicationException("Failed to deserialize object.", e);
        }
    }
}
