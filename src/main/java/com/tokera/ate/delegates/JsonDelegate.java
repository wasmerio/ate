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

    public String serialize(Object obj) {
        ObjectMapper mapper = new ObjectMapper();
        try {
            return mapper.writeValueAsString(obj);
        } catch (JsonProcessingException e) {
            throw new WebApplicationException("Failed to serialize object.", e);
        }
    }

    public <T> Object deserialize(String data, Class<T> clazz) {
        ObjectMapper mapper = new ObjectMapper();
        try {
            return mapper.readValue(data, clazz);
        } catch (IOException e) {
            throw new WebApplicationException("Failed to deserialize object.", e);
        }
    }
}
