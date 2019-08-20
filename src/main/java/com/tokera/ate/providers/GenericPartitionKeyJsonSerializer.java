package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.dao.GenericPartitionKey;

import java.io.IOException;

public class GenericPartitionKeyJsonSerializer extends StdScalarSerializer<GenericPartitionKey> {
    public GenericPartitionKeyJsonSerializer() {
        super(GenericPartitionKey.class);
    }
    protected GenericPartitionKeyJsonSerializer(Class<GenericPartitionKey> t) {
        super(t);
    }

    @Override
    public void serialize(GenericPartitionKey value, JsonGenerator gen, SerializerProvider provider) throws IOException {
        GenericPartitionKeySerializer serializer = new GenericPartitionKeySerializer();
        gen.writeString(serializer.write(value));
    }
}
