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

import java.util.BitSet;

import static com.googlecode.ipv6.BitSetHelpers.bitSetOf;

/**
 * Helper methods used by IPv6Network.
 *
 * @author Jan Van Besien
 */
@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public class IPv6NetworkHelpers
{
    static int longestPrefixLength(IPv6Address first, IPv6Address last)
    {
        final BitSet firstBits = bitSetOf(first.getLowBits(), first.getHighBits());
        final BitSet lastBits = bitSetOf(last.getLowBits(), last.getHighBits());

        return countLeadingSimilarBits(firstBits, lastBits);
    }

    private static int countLeadingSimilarBits(BitSet a, BitSet b)
    {
        int result = 0;
        for (int i = 127; i >= 0 && (a.get(i) == b.get(i)); i--)
        {
            result++;
        }

        return result;
    }
}