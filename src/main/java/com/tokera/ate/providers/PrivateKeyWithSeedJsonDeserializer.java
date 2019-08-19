package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.TokenDto;

import java.io.IOException;

public class PrivateKeyWithSeedJsonDeserializer extends StdScalarDeserializer<PrivateKeyWithSeedDto> {
    public PrivateKeyWithSeedJsonDeserializer() {
        super(PrivateKeyWithSeedDto.class);
    }
    protected PrivateKeyWithSeedJsonDeserializer(Class<PrivateKeyWithSeedDto> t) {
        super(t);
    }

    @Override
    public PrivateKeyWithSeedDto deserialize(JsonParser p, DeserializationContext ctxt) throws IOException, JsonProcessingException {
        return PrivateKeyWithSeedDto.deserialize(p.getValueAsString());
    }
}
