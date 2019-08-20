package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonParser;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationContext;
import com.fasterxml.jackson.databind.deser.std.StdScalarDeserializer;
import com.tokera.ate.dao.GenericPartitionKey;

import java.io.IOException;

public class GenericPartitionKeyJsonDeserializer extends StdScalarDeserializer<GenericPartitionKey> {
    public GenericPartitionKeyJsonDeserializer() {
        super(GenericPartitionKey.class);
    }
    protected GenericPartitionKeyJsonDeserializer(Class<GenericPartitionKey> t) {
        super(t);
    }

    @Override
    public GenericPartitionKey deserialize(JsonParser p, DeserializationContext ctxt) throws IOException, JsonProcessingException {
        GenericPartitionKeySerializer serializer = new GenericPartitionKeySerializer();
        return serializer.read(p.getValueAsString());
    }
}