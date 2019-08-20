package com.tokera.ate.dao;

import com.fasterxml.jackson.annotation.JsonTypeName;
import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.providers.CountLongJsonDeserializer;
import com.tokera.ate.providers.CountLongJsonSerializer;

import javax.enterprise.context.Dependent;
import java.io.Serializable;
import java.util.concurrent.atomic.AtomicLong;

@Dependent
@YamlTag("count.long")
@JsonTypeName("count.long")
@JsonSerialize(using = CountLongJsonSerializer.class)
@JsonDeserialize(using = CountLongJsonDeserializer.class)
public final class CountLong extends AtomicLong implements Serializable, Comparable<CountLong> {
    private static final long serialVersionUID = -572858007703415022L;

    public CountLong() {
        super();
    }

    public CountLong(long number) {
        super(number);
    }

    public boolean equals(Object obj) {
        if (obj == this) return true;
        if (obj instanceof CountLong == false) return false;
        CountLong other = (CountLong) obj;
        return this.get() == other.get();
    }

    public int hashCode() {
        long val = get();
        int hashCode = 17;
        hashCode = 37 * hashCode + getClass().hashCode();
        hashCode = 37 * hashCode + ((int) (val ^ (val >> 32)));
        return hashCode;
    }

    public String toString() {
        return Long.toString(this.get());
    }

    @Override
    public int compareTo(CountLong other) {
        return Long.compare(this.get(), other.get());
    }

    public Long value() {
        return get();
    }

    public Long increment() {
        return super.incrementAndGet();
    }

    public Long decrement() {
        return super.decrementAndGet();
    }

    public static CountLong parse(String val) {
        Long ret = Long.parseLong(val);
        return new CountLong(ret);
    }

    public static String serialize(CountLong val) {
        return Long.toString(val.get());
    }

    public static CountLong clone(Object other) {
        if (other == null) {
            return null;
        } else if (other instanceof Number) {
            return new CountLong(((Number)other).longValue());
        } else {
            return new CountLong(0L);
        }
    }
}
