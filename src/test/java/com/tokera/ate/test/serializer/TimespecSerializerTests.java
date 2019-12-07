package com.tokera.ate.test.serializer;

import com.tokera.ate.dto.Timespec;
import com.tokera.ate.providers.TimespecSerializer;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

import java.util.Date;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class TimespecSerializerTests {

    @Test
    public void dateAndBack() {
        Timespec spec1 = new Timespec(121L, 41L * 1000L * 1000L);
        Date date1 = spec1.toDate();

        Timespec spec2 = new Timespec(date1);
        Date date2 = spec2.toDate();

        Assertions.assertEquals(spec1, spec2);
        Assertions.assertEquals(date1, date2);
    }

    @Test
    public void dateSerializers() {
        Timespec spec1 = new Timespec(121L, 41L * 1000L * 1000L);

        TimespecSerializer serializer = new TimespecSerializer();
        String data1 = serializer.write(spec1);

        Timespec spec2 = serializer.read(data1);
        String data2 = serializer.write(spec2);

        Assertions.assertEquals(spec1, spec2);
        Assertions.assertEquals(data1, data2);
    }
}
