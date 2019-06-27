package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.dao.RangeLong;

import java.io.IOException;

public class RangeLongJsonSerializer extends StdScalarSerializer<RangeLong> {
    private RangeLongSerializer serializer = new RangeLongSerializer();

    public RangeLongJsonSerializer() {
        super(RangeLong.class);
    }
    protected RangeLongJsonSerializer(Class<RangeLong> t) {
        super(t);
    }

    @Override
    public void serialize(RangeLong value, JsonGenerator gen, SerializerProvider provider) throws IOException {
        gen.writeString(serializer.write(value));
    }
}
