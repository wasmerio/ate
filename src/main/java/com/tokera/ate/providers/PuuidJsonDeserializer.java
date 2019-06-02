package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.tokera.ate.dao.PUUID;

import java.io.IOException;

public class PuuidJsonDeserializer extends StdScalarDeserializer<PUUID> {
    public PuuidJsonDeserializer() {
        super(PUUID.class);
    }
    protected PuuidJsonDeserializer(Class<PUUID> t) {
        super(t);
    }

    @Override
    public PUUID deserialize(JsonParser p, DeserializationContext ctxt) throws IOException, JsonProcessingException {
        return PUUID.parse(p.getValueAsString());
    }
}
