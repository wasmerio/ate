package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.tokera.ate.dao.CountLong;

import java.io.IOException;

public class CountLongJsonDeserializer extends StdScalarDeserializer<CountLong> {
    public CountLongJsonDeserializer() {
        super(CountLong.class);
    }
    protected CountLongJsonDeserializer(Class<CountLong> t) {
        super(t);
    }

    @Override
    public CountLong deserialize(JsonParser p, DeserializationContext ctxt) throws IOException, JsonProcessingException {
        return CountLong.parse(p.getValueAsString());
    }
}
