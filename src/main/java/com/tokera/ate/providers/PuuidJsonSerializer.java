package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.dao.PUUID;

import java.io.IOException;

public class PuuidJsonSerializer extends StdScalarSerializer<PUUID> {
    public PuuidJsonSerializer() {
        super(PUUID.class);
    }
    protected PuuidJsonSerializer(Class<PUUID> t) {
        super(t);
    }

    @Override
    public void serialize(PUUID value, JsonGenerator gen, SerializerProvider provider) throws IOException {
        gen.writeString(PUUID.serialize(value));
    }
}
