package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.tokera.ate.io.api.IPartitionKey;

import java.io.IOException;

public class PartitionKeyJsonDeserializer extends StdScalarDeserializer<IPartitionKey> {
    public PartitionKeyJsonDeserializer() {
        super(IPartitionKey.class);
    }
    protected PartitionKeyJsonDeserializer(Class<IPartitionKey> t) {
        super(t);
    }

    @Override
    public IPartitionKey deserialize(JsonParser p, DeserializationContext ctxt) throws IOException, JsonProcessingException {
        PartitionKeySerializer serializer = new PartitionKeySerializer();
        return serializer.read(p.getValueAsString());
    }
}