package com.tokera.ate.providers;

import com.fasterxml.jackson.core.JsonGenerator;
import com.fasterxml.jackson.databind.SerializerProvider;
import com.fasterxml.jackson.databind.ser.std.StdScalarSerializer;
import com.tokera.ate.io.api.IPartitionKey;

import java.io.IOException;

public class PartitionKeyJsonSerializer extends StdScalarSerializer<IPartitionKey> {
    public PartitionKeyJsonSerializer() {
        super(IPartitionKey.class);
    }
    protected PartitionKeyJsonSerializer(Class<IPartitionKey> t) {
        super(t);
    }

    @Override
    public void serialize(IPartitionKey value, JsonGenerator gen, SerializerProvider provider) throws IOException {
        PartitionKeySerializer serializer = new PartitionKeySerializer();
        gen.writeString(serializer.write(value));
    }
}
