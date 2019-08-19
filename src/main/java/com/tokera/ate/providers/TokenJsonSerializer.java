package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dto.TokenDto;

import java.io.IOException;

public class TokenJsonSerializer extends StdScalarSerializer<TokenDto> {
    public TokenJsonSerializer() {
        super(TokenDto.class);
    }
    protected TokenJsonSerializer(Class<TokenDto> t) {
        super(t);
    }

    @Override
    public void serialize(TokenDto value, JsonGenerator gen, SerializerProvider provider) throws IOException {
        gen.writeString(value.getBase64());
    }
}
