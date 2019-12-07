package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.tokera.ate.dto.Timespec;

import java.io.IOException;

public class TimespecJsonDeserializer extends StdScalarDeserializer<Timespec> {
    private TimespecSerializer serializer = new TimespecSerializer();

    public TimespecJsonDeserializer() {
        super(Timespec.class);
    }
    protected TimespecJsonDeserializer(Class<Timespec> t) {
        super(t);
    }

    @Override
    public Timespec deserialize(JsonParser p, DeserializationContext ctxt) throws IOException {
        return serializer.read(p.getValueAsString());
    }
}
