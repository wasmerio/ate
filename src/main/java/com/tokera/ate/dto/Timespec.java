package com.tokera.ate.dto;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.annotation.JsonTypeName;
import com.fasterxml.jackson.databind.annotation.JsonDeserialize;
import com.fasterxml.jackson.databind.annotation.JsonSerialize;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.providers.TimespecJsonDeserializer;
import com.tokera.ate.providers.TimespecJsonSerializer;
import com.tokera.ate.providers.TokenJsonDeserializer;
import com.tokera.ate.providers.TokenJsonSerializer;
import org.apache.commons.lang3.StringUtils;

import javax.enterprise.context.Dependent;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.time.LocalDateTime;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.time.ZonedDateTime;
import java.time.format.DateTimeFormatter;
import java.util.Calendar;
import java.util.Date;
import java.util.TimeZone;

@Dependent
@YamlTag("timespec")
@JsonTypeName("timespec")
@JsonSerialize(using = TimespecJsonSerializer.class)
@JsonDeserialize(using = TimespecJsonDeserializer.class)
public class Timespec implements Comparable<Timespec> {

    @JsonProperty
    public long tv_sec;
    @JsonProperty
    public long tv_nsec;

    public Timespec() {
    }

    public Timespec(long sec, long nsec) {
        this.tv_sec = sec;
        this.tv_nsec = nsec;
    }

    public Timespec(ByteBuffer buffer) {
        buffer.order(ByteOrder.nativeOrder());
        this.tv_sec = buffer.getLong();
        this.tv_nsec = buffer.getLong();
    }

    public Timespec(Date when) {
        long epochTime = when.toInstant().toEpochMilli();
        long milliseconds = epochTime % 1000L;
        long seconds = epochTime / 1000L;

        this.tv_sec = seconds;
        this.tv_nsec = milliseconds * 1000L * 1000L;
    }

    public DateInfo toDateInfo() {
        return new DateInfo(this.tv_sec);
    }

    public TimeInfo toTimeInfo() {
        return new TimeInfo(this.tv_sec, this.tv_nsec);
    }

    public Date toDate() {
        long millis = (this.tv_sec * 1000L) + (this.tv_nsec / (1000L * 1000L));
        return new Date(millis);
    }

    @Override
    public int compareTo(Timespec other) {
        long sec_diff = this.tv_sec - other.tv_sec;
        if (sec_diff > 0) {
            return 1;
        } else if (sec_diff < 0) {
            return -1;
        }
        long nsec_diff = this.tv_nsec - other.tv_nsec;
        if (nsec_diff > 0) {
            return 1;
        } else if (nsec_diff < 0) {
            return -1;
        }
        return 0;
    }

    @Override
    public boolean equals(Object obj) {
        if (obj == this) {
            return true;
        }
        if (obj == null || this.getClass() != obj.getClass()) {
            return false;
        }
        return this.equals((Timespec) obj);
    }

    public boolean equals(Timespec other) {
        if (other == this) {
            return true;
        }
        if (other == null) {
            return false;
        }
        return this.tv_sec == other.tv_sec
                && this.tv_nsec == other.tv_nsec;
    }

    @Override
    public int hashCode() {
        int hash = 5;
        hash = 17 * hash + (int) (this.tv_sec);
        hash = 17 * hash + (int) (this.tv_nsec);
        return hash;
    }

    public LocalDateTime toLocalDateTime() {
        return LocalDateTime.ofEpochSecond(this.tv_sec, (int) this.tv_nsec, ZoneOffset.UTC);
    }

    public OffsetDateTime toOffsetDateTime() {
        return OffsetDateTime.of(this.toLocalDateTime(), ZoneOffset.UTC);
    }

    @Override
    public String toString() {
        return this.toDateInfo() + "T" + this.toTimeInfo() + "Z";
    }

    public static final class DateInfo {

        public final int year;
        public final int month;
        public final int day;

        public DateInfo(long seconds) {
            int epochDays = (int) (seconds / 86400L);
            epochDays += 719468;
            int era = (epochDays >= 0 ? epochDays : epochDays - 146096) / 146097;
            int dayOfEra = epochDays - era * 146097;
            int yearOfEra = (dayOfEra - dayOfEra / 1460 + dayOfEra / 36524 - dayOfEra / 146096) / 365;
            int y = yearOfEra + era * 400;
            int dayOfYear = dayOfEra - (365 * yearOfEra + yearOfEra / 4 - yearOfEra / 100);
            int mp = (5 * dayOfYear + 2) / 153;
            int d = dayOfYear - (153 * mp + 2) / 5 + 1;
            int m = mp + (mp < 10 ? 3 : -9);
            this.year = y + (m <= 2 ? 1 : 0);
            this.month = m;
            this.day = d;
        }

        @Override
        public String toString() {
            StringBuilder sb = new StringBuilder("9999-99-99".length());
            sb.append(StringUtils.leftPad(String.valueOf(this.year), 4, '0'));
            sb.append('-');
            sb.append(StringUtils.leftPad(String.valueOf(this.month), 2, '0'));
            sb.append('-');
            sb.append(StringUtils.leftPad(String.valueOf(this.day), 2, '0'));
            return sb.toString();
        }
    }

    public static final class TimeInfo {

        public final int hour;
        public final int minute;
        public final int second;
        public final int millisecond;

        public TimeInfo(long seconds, long nanoseconds) {
            this.millisecond = (int) (nanoseconds / 1000000L);
            int remainingSeconds = (int) (seconds % 86400L);
            this.hour = remainingSeconds / 3600;
            remainingSeconds -= this.hour * 3600;
            this.minute = remainingSeconds / 60;
            remainingSeconds -= this.minute * 60;
            this.second = remainingSeconds;
        }

        @Override
        public String toString() {
            StringBuilder sb = new StringBuilder("99:99:99.999".length());
            sb.append(StringUtils.leftPad(String.valueOf(this.hour), 2, '0'));
            sb.append(':');
            sb.append(StringUtils.leftPad(String.valueOf(this.minute), 2, '0'));
            sb.append(':');
            sb.append(StringUtils.leftPad(String.valueOf(this.second), 2, '0'));
            sb.append('.');
            sb.append(StringUtils.leftPad(String.valueOf(this.millisecond), 3, '0'));
            return sb.toString();
        }
    }

}
