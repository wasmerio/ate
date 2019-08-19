package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.TokenDto;

import java.io.IOException;

public class PrivateKeyWithSeedJsonSerializer extends StdScalarSerializer<PrivateKeyWithSeedDto> {
    public PrivateKeyWithSeedJsonSerializer() {
        super(PrivateKeyWithSeedDto.class);
    }
    protected PrivateKeyWithSeedJsonSerializer(Class<PrivateKeyWithSeedDto> t) {
        super(t);
    }

    @Override
    public void serialize(PrivateKeyWithSeedDto value, JsonGenerator gen, SerializerProvider provider) throws IOException {
        gen.writeString(value.serialize());
    }
}
