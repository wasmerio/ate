package com.tokera.ate.dao;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.providers.RangeLongJsonDeserializer;
import com.tokera.ate.providers.RangeLongJsonSerializer;
import org.apache.commons.lang.math.Range;
import org.apache.commons.lang.text.StrBuilder;

import java.io.Serializable;

@YamlTag("puuid")
@JsonSerialize(using = RangeLongJsonSerializer.class)
@JsonDeserialize(using = RangeLongJsonDeserializer.class)
public final class RangeLong extends Range implements Serializable, Comparable<RangeLong> {
    private static final long serialVersionUID = -8412288864327818063L;

    @JsonProperty
    private long min;
    @JsonProperty
    private long max;

    public RangeLong() {
        super();
        this.min = 0;
        this.max = 0;
    }

    /**
     * <p>Constructs a new <code>LongRange</code> using the specified
     * number as both the minimum and maximum in this range.</p>
     *
     * @param number  the number to use for this range
     */
    public RangeLong(long number) {
        super();
        this.min = number;
        this.max = number;
    }

    /**
     * <p>Constructs a new <code>LongRange</code> using the specified
     * number as both the minimum and maximum in this range.</p>
     *
     * @param number  the number to use for this range, must not
     *  be <code>null</code>
     * @throws IllegalArgumentException if the number is <code>null</code>
     */
    public RangeLong(Number number) {
        super();
        if (number == null) {
            throw new IllegalArgumentException("The number must not be null");
        }
        this.min = number.longValue();
        this.max = number.longValue();
    }

    /**
     * <p>Constructs a new <code>LongRange</code> with the specified
     * minimum and maximum numbers (both inclusive).</p>
     *
     * <p>The arguments may be passed in the order (min,max) or (max,min). The
     * getMinimum and getMaximum methods will return the correct values.</p>
     *
     * @param number1  first number that defines the edge of the range, inclusive
     * @param number2  second number that defines the edge of the range, inclusive
     */
    public RangeLong(long number1, long number2) {
        super();
        if (number2 < number1) {
            this.min = number2;
            this.max = number1;
        } else {
            this.min = number1;
            this.max = number2;
        }
    }

    /**
     * <p>Constructs a new <code>LongRange</code> with the specified
     * minimum and maximum numbers (both inclusive).</p>
     *
     * <p>The arguments may be passed in the order (min,max) or (max,min). The
     * getMinimum and getMaximum methods will return the correct values.</p>
     *
     * @param number1  first number that defines the edge of the range, inclusive
     * @param number2  second number that defines the edge of the range, inclusive
     * @throws IllegalArgumentException if either number is <code>null</code>
     */
    public RangeLong(Number number1, Number number2) {
        super();
        if (number1 == null || number2 == null) {
            throw new IllegalArgumentException("The numbers must not be null");
        }
        long number1val = number1.longValue();
        long number2val = number2.longValue();
        if (number2val < number1val) {
            this.min = number2val;
            this.max = number1val;
        } else {
            this.min = number1val;
            this.max = number2val;
        }
    }

    // Accessors
    //--------------------------------------------------------------------

    /**
     * <p>Returns the minimum number in this range.</p>
     *
     * @return the minimum number in this range
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public Number getMinimumNumber() {
        return new Long(min);
    }

    /**
     * <p>Gets the minimum number in this range as a <code>long</code>.</p>
     *
     * @return the minimum number in this range
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public long getMinimumLong() {
        return min;
    }

    /**
     * <p>Gets the minimum number in this range as a <code>int</code>.</p>
     *
     * <p>This conversion can lose information for large values.</p>
     *
     * @return the minimum number in this range
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public int getMinimumInteger() {
        return (int) min;
    }

    /**
     * <p>Gets the minimum number in this range as a <code>double</code>.</p>
     *
     * <p>This conversion can lose information for large values.</p>
     *
     * @return the minimum number in this range
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public double getMinimumDouble() {
        return min;
    }

    /**
     * <p>Gets the minimum number in this range as a <code>float</code>.</p>
     *
     * <p>This conversion can lose information for large values.</p>
     *
     * @return the minimum number in this range
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public float getMinimumFloat() {
        return min;
    }

    /**
     * <p>Returns the maximum number in this range.</p>
     *
     * @return the maximum number in this range
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public Number getMaximumNumber() {
        return new Long(max);
    }

    /**
     * <p>Gets the maximum number in this range as a <code>long</code>.</p>
     *
     * @return the maximum number in this range
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public long getMaximumLong() {
        return max;
    }

    /**
     * <p>Gets the maximum number in this range cast to an <code>int</code>.</p>
     *
     * <p>This conversion can lose information for large values.</p>
     *
     * @return the maximum number in this range cast to an <code>int</code>.
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public int getMaximumInteger() {
        return (int) max;
    }

    /**
     * <p>Gets the maximum number in this range as a <code>double</code>.</p>
     *
     * <p>This conversion can lose information for large values.</p>
     *
     * @return The maximum number in this range as a <code>double</code>.
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public double getMaximumDouble() {
        return max;
    }

    /**
     * <p>Gets the maximum number in this range as a <code>float</code>.</p>
     *
     * <p>This conversion can lose information for large values.</p>
     *
     * @return The maximum number in this range as a <code>float</code>.
     */
    @com.jsoniter.annotation.JsonIgnore
    @JsonIgnore
    public float getMaximumFloat() {
        return max;
    }

    // Tests
    //--------------------------------------------------------------------

    /**
     * <p>Tests whether the specified <code>number</code> occurs within
     * this range using <code>long</code> comparison.</p>
     *
     * <p><code>null</code> is handled and returns <code>false</code>.</p>
     *
     * @param number  the number to test, may be <code>null</code>
     * @return <code>true</code> if the specified number occurs within this range
     */
    public boolean containsNumber(Number number) {
        if (number == null) {
            return false;
        }
        return containsLong(number.longValue());
    }

    /**
     * <p>Tests whether the specified <code>long</code> occurs within
     * this range using <code>long</code> comparison.</p>
     *
     * <p>This implementation overrides the superclass for performance as it is
     * the most common case.</p>
     *
     * @param value  the long to test
     * @return <code>true</code> if the specified number occurs within this
     *  range by <code>long</code> comparison
     */
    public boolean containsLong(long value) {
        return value >= min && value <= max;
    }

    // Range tests
    //--------------------------------------------------------------------

    /**
     * <p>Tests whether the specified range occurs entirely within this range
     * using <code>long</code> comparison.</p>
     *
     * <p><code>null</code> is handled and returns <code>false</code>.</p>
     *
     * @param range  the range to test, may be <code>null</code>
     * @return <code>true</code> if the specified range occurs entirely within this range
     * @throws IllegalArgumentException if the range is not of this type
     */
    public boolean containsRange(Range range) {
        if (range == null) {
            return false;
        }
        return containsLong(range.getMinimumLong()) &&
                containsLong(range.getMaximumLong());
    }

    /**
     * <p>Tests whether the specified range overlaps with this range
     * using <code>long</code> comparison.</p>
     *
     * <p><code>null</code> is handled and returns <code>false</code>.</p>
     *
     * @param range  the range to test, may be <code>null</code>
     * @return <code>true</code> if the specified range overlaps with this range
     */
    public boolean overlapsRange(Range range) {
        if (range == null) {
            return false;
        }
        return range.containsLong(min) ||
                range.containsLong(max) ||
                containsLong(range.getMinimumLong());
    }

    // Basics
    //--------------------------------------------------------------------

    /**
     * <p>Compares this range to another object to test if they are equal.</p>.
     *
     * <p>To be equal, the class, minimum and maximum must be equal.</p>
     *
     * @param obj the reference object with which to compare
     * @return <code>true</code> if this object is equal
     */
    public boolean equals(Object obj) {
        if (obj == this) {
            return true;
        }
        if (obj instanceof RangeLong == false) {
            return false;
        }
        RangeLong range = (RangeLong) obj;
        return min == range.min && max == range.max;
    }

    /**
     * <p>Gets a hashCode for the range.</p>
     *
     * @return a hash code value for this object
     */
    public int hashCode() {
        int hashCode = 17;
        hashCode = 37 * hashCode + getClass().hashCode();
        hashCode = 37 * hashCode + ((int) (min ^ (min >> 32)));
        hashCode = 37 * hashCode + ((int) (max ^ (max >> 32)));
        return hashCode;
    }

    /**
     * <p>Gets the range as a <code>String</code>.</p>
     *
     * <p>The format of the String is 'Range[<i>min</i>,<i>max</i>]'.</p>
     *
     * @return the <code>String</code> representation of this range
     */
    public String toString() {
        StrBuilder buf = new StrBuilder(32);
        buf.append(min);
        buf.append(':');
        buf.append(max);
        return buf.toString();
    }

    /**
     * <p>Returns an array containing all the long values in the range.</p>
     *
     * @return the <code>long[]</code> representation of this range
     * @since 2.4
     */
    public long[] toArray() {
        long[] array = new long[(int)(max - min + 1L)];
        for(int i = 0; i < array.length; i++) {
            array[i] = min + i;
        }
        return array;
    }

    @Override
    public int compareTo(RangeLong other) {
        int ret = Long.compare(this.min, other.min);
        if (ret != 0) return ret;
        return Long.compare(this.max, other.max);
    }

    /**
     * The minimum number in this range (inclusive).
     */
    public long getMin() {
        return min;
    }

    public void setMin(long min) {
        this.min = min;
    }

    /**
     * The maximum number in this range (inclusive).
     */
    public long getMax() {
        return max;
    }

    public void setMax(long max) {
        this.max = max;
    }
}
