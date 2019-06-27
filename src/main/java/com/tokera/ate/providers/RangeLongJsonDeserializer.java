package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.tokera.ate.dao.RangeLong;

import java.io.IOException;

public class RangeLongJsonDeserializer extends StdScalarDeserializer<RangeLong> {
    private RangeLongSerializer serializer = new RangeLongSerializer();

    public RangeLongJsonDeserializer() {
        super(RangeLong.class);
    }
    protected RangeLongJsonDeserializer(Class<RangeLong> t) {
        super(t);
    }

    @Override
    public RangeLong deserialize(JsonParser p, DeserializationContext ctxt) throws IOException {
        return serializer.read(p.getValueAsString());
    }
}
