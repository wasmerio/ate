package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dto.TokenDto;

import java.io.IOException;

public class TokenJsonDeserializer extends StdScalarDeserializer<TokenDto> {
    public TokenJsonDeserializer() {
        super(TokenDto.class);
    }
    protected TokenJsonDeserializer(Class<TokenDto> t) {
        super(t);
    }

    @Override
    public TokenDto deserialize(JsonParser p, DeserializationContext ctxt) throws IOException, JsonProcessingException {
        return new TokenDto(p.getValueAsString());
    }
}
