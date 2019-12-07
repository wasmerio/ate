package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.dto.Timespec;

import java.io.IOException;

public class TimespecJsonSerializer extends StdScalarSerializer<Timespec> {
    private TimespecSerializer serializer = new TimespecSerializer();

    public TimespecJsonSerializer() {
        super(Timespec.class);
    }
    protected TimespecJsonSerializer(Class<Timespec> t) {
        super(t);
    }

    @Override
    public void serialize(Timespec value, JsonGenerator gen, SerializerProvider provider) throws IOException {
        gen.writeString(serializer.write(value));
    }
}
