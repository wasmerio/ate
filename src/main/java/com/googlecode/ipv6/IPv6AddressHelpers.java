/*
 * Copyright 2013 Jan Van Besien
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package com.googlecode.ipv6;

import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;

import java.util.Arrays;
import java.util.regex.Pattern;

/**
 * Helper methods used by IPv6Address.
 *
 * @author Jan Van Besien
 */
@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public final class IPv6AddressHelpers
{
    static long[] parseStringArrayIntoLongArray(String[] strings)
    {
        final long[] longs = new long[strings.length];
        for (int i = 0; i < strings.length; i++)
        {
            longs[i] = Long.parseLong(strings[i], 16);
        }
        return longs;
    }

    static void validateLongs(long[] longs)
    {
        if (longs.length != 8)
            throw new IllegalArgumentException("an IPv6 address should contain 8 shorts [" + Arrays.toString(longs) + "]");

        for (long l : longs)
        {
            if (l < 0) throw new IllegalArgumentException("each element should be positive [" + Arrays.toString(longs) + "]");
            if (l > 0xFFFF) throw new IllegalArgumentException("each element should be less than 0xFFFF [" + Arrays.toString(longs) + "]");
        }
    }

    static IPv6Address mergeLongArrayIntoIPv6Address(long[] longs)
    {
        long high = 0L;
        long low = 0L;

        for (int i = 0; i < longs.length; i++)
        {
            if (inHighRange(i))
                high |= (longs[i] << ((longs.length - i - 1) * 16));
            else
                low |= (longs[i] << ((longs.length - i - 1) * 16));
        }

        return new IPv6Address(high, low);
    }

    static boolean inHighRange(int shortNumber)
    {
        return shortNumber >= 0 && shortNumber < 4;
    }

    static String expandShortNotation(String string)
    {
        if (!string.contains("::"))
        {
            return string;
        }
        else if (string.equals("::"))
        {
            return generateZeroes(8);
        }
        else
        {
            final int numberOfColons = countOccurrences(string, ':');
            if (string.startsWith("::"))
                return string.replace("::", generateZeroes((7 + 2) - numberOfColons));
            else if (string.endsWith("::"))
                return string.replace("::", ":" + generateZeroes((7 + 2) - numberOfColons));
            else
                return string.replace("::", ":" + generateZeroes((7 + 2 - 1) - numberOfColons));
        }
    }

    private static final Pattern DOT_DELIM = Pattern.compile("\\.");

    /**
     * Replaces a w.x.y.z substring at the end of the given string with corresponding hexadecimal notation. This is useful in case the
     * string was using IPv4-Mapped address notation.
     */
    static String rewriteIPv4MappedNotation(String string)
    {
        if (!string.contains("."))
        {
            return string;
        }
        else
        {
            int lastColon = string.lastIndexOf(":");
            String firstPart = string.substring(0, lastColon + 1);
            String mappedIPv4Part = string.substring(lastColon + 1);

            if (mappedIPv4Part.contains("."))
            {
                String[] dotSplits = DOT_DELIM.split(mappedIPv4Part);
                if (dotSplits.length != 4)
                    throw new IllegalArgumentException(String.format("can not parse [%s]", string));

                StringBuilder rewrittenString = new StringBuilder();
                rewrittenString.append(firstPart);
                int byteZero = Integer.parseInt(dotSplits[0]);
                int byteOne = Integer.parseInt(dotSplits[1]);
                int byteTwo = Integer.parseInt(dotSplits[2]);
                int byteThree = Integer.parseInt(dotSplits[3]);

                rewrittenString.append(String.format("%02x", byteZero));
                rewrittenString.append(String.format("%02x", byteOne));
                rewrittenString.append(":");
                rewrittenString.append(String.format("%02x", byteTwo));
                rewrittenString.append(String.format("%02x", byteThree));

                return rewrittenString.toString();
            }
            else
            {
                throw new IllegalArgumentException(String.format("can not parse [%s]", string));
            }
        }
    }

    public static int countOccurrences(String haystack, char needle)
    {
        int count = 0;
        for (int i = 0; i < haystack.length(); i++)
        {
            if (haystack.charAt(i) == needle)
            {
                count++;
            }
        }
        return count;
    }

    public static String generateZeroes(int number)
    {
        final StringBuilder builder = new StringBuilder();
        for (int i = 0; i < number; i++)
        {
            builder.append("0:");
        }

        return builder.toString();
    }

    static boolean isLessThanUnsigned(long a, long b)
    {
        return (a < b) ^ ((a < 0) != (b < 0));
    }

    static byte[] prefixWithZeroBytes(byte[] original, int newSize)
    {
        byte[] target = new byte[newSize];
        System.arraycopy(original, 0, target, newSize - original.length, original.length);
        return target;
    }
}