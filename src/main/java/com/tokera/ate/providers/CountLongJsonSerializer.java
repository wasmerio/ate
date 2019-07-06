package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.dao.CountLong;

import java.io.IOException;

public class CountLongJsonSerializer extends StdScalarSerializer<CountLong> {
    public CountLongJsonSerializer() {
        super(CountLong.class);
    }
    protected CountLongJsonSerializer(Class<CountLong> t) {
        super(t);
    }

    @Override
    public void serialize(CountLong value, JsonGenerator gen, SerializerProvider provider) throws IOException {
        gen.writeString(CountLong.serialize(value));
    }
}
