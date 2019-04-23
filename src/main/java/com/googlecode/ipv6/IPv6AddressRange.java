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

import org.checkerframework.checker.nullness.qual.NonNull;
import org.checkerframework.checker.nullness.qual.Nullable;
import org.checkerframework.framework.qual.DefaultQualifier;

import java.math.BigInteger;
import java.util.*;

/**
 * Immutable representation of a continuous range of IPv6 addresses (bounds included).
 *
 * @author Jan Van Besien
 */
@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public class IPv6AddressRange implements Comparable<IPv6AddressRange>, Iterable<IPv6Address>
{
    private final IPv6Address first;

    private final IPv6Address last;

    IPv6AddressRange(@NonNull IPv6Address first, @NonNull IPv6Address last)
    {
        if (first.compareTo(last) > 0)
            throw new IllegalArgumentException("Cannot create ip address range with last address < first address");

        this.first = first;
        this.last = last;
    }

    public static IPv6AddressRange fromFirstAndLast(IPv6Address first, IPv6Address last)
    {
        return new IPv6AddressRange(first, last);
    }

    public boolean contains(IPv6Address address)
    {
        return first.compareTo(address) <= 0 && last.compareTo(address) >= 0;
    }

    public boolean contains(IPv6AddressRange range)
    {
        return contains(range.first) && contains(range.last);
    }

    public boolean overlaps(IPv6AddressRange range)
    {
        return contains(range.first) || contains(range.last) || range.contains(first) || range.contains(last);
    }

    /**
     * @return an iterator which iterates all addresses in this range, in order.
     */
    @Override
    public Iterator<IPv6Address> iterator()
    {
        return new IPv6AddressRangeIterator();
    }

    /**
     * @return number of addresses in the range
     */
    public BigInteger size()
    {
        BigInteger firstAsBigInteger = new BigInteger(1, first.toByteArray());
        BigInteger lastAsBigInteger = new BigInteger(1, last.toByteArray());

        // note that first and last are included in the range.
        return lastAsBigInteger.subtract(firstAsBigInteger).add(BigInteger.ONE);
    }

    /**
     * Deaggregate a range of IPv6 addresses (which is not necessarily aligned with a single IPv6 network) into a minimal set of non
     * overlapping consecutive subnets.
     *
     * @return iterator of IPv6 networks that all together define the minimal set of subnets by which the range can be represented.
     */
    public Iterator<IPv6Network> toSubnets()
    {
        return new IPv6AddressRangeAsSubnetsIterator();
    }

    /**
     * Remove an address from the range, resulting in one, none or two new ranges. If an address outside the range is removed, this has no
     * effect. If the first or last address is removed, a single new range is returned (potentially empty if the range only contained a
     * single address). If an address somewhere else in the range is removed, two new ranges are returned.
     *
     * @param address adddress to remove from the range
     * @return list of resulting ranges
     */
    public List<IPv6AddressRange> remove(IPv6Address address)
    {
        if (address == null)
            throw new IllegalArgumentException("invalid address [null]");

        if (!contains(address))
            return Collections.singletonList(this);
        else if (address.equals(first) && address.equals(last))
            return Collections.emptyList();
        else if (address.equals(first))
            return Collections.singletonList(fromFirstAndLast(first.add(1), last));
        else if (address.equals(last))
            return Collections.singletonList(fromFirstAndLast(first, last.subtract(1)));
        else
            return Arrays.asList(fromFirstAndLast(first, address.subtract(1)),
                                 fromFirstAndLast(address.add(1), last));
    }

    /**
     * Extend the range just enough at its head or tail such that the given address is included.
     *
     * @param address address to extend the range to
     * @return new (bigger) range
     */
    public IPv6AddressRange extend(IPv6Address address)
    {
        if (address.compareTo(first) < 0)
            return fromFirstAndLast(address, last);
        else if (address.compareTo(last) > 0)
            return fromFirstAndLast(first, address);
        else
            return this;
    }

    /**
     * Remove a network from the range, resulting in one, none or two new ranges. If a network outside (or partially outside) the range is
     * removed, this has no effect. If the network which is removed is aligned with the beginning or end of the range, a single new ranges
     * is returned (potentially empty if the range was equal to the network which is removed from it). If a network somewhere else in the
     * range is removed, two new ranges are returned.
     *
     * @param network network to remove from the range
     * @return list of resulting ranges
     */
    public List<IPv6AddressRange> remove(IPv6Network network)
    {
        if (network == null)
            throw new IllegalArgumentException("invalid network [null]");

        if (!contains(network))
            return Collections.singletonList(this);
        else if (this.equals(network))
            return Collections.emptyList();
        else if (first.equals(network.getFirst()))
            return Collections.singletonList(fromFirstAndLast(network.getLast().add(1), last));
        else if (last.equals(network.getLast()))
            return Collections.singletonList(fromFirstAndLast(first, network.getFirst().subtract(1)));
        else
            return Arrays.asList(fromFirstAndLast(first, network.getFirst().subtract(1)),
                                 fromFirstAndLast(network.getLast().add(1), last));

    }

    @Override
    public String toString()
    {
        return first.toString() + " - " + last.toString();
    }

    /**
     * @return like <code>toString</code> but without using shorthand notations for addresses
     */
    public String toLongString()
    {
        return first.toLongString() + " - " + last.toLongString();
    }

    /**
     * The natural order of {@link com.googlecode.ipv6.IPv6AddressRange}s orders them on increasing first addresses, and on increasing last
     * address if the first address would be equal.
     * <p/>
     * Note that the natural order does thus not compare sizes of ranges.
     *
     * @param that range to compare with
     * @return negative, zero or positive depending on whether this is smaller, equal or greater than that
     */
    @Override
    public int compareTo(IPv6AddressRange that)
    {
        if (!this.first.equals(that.first))
            return this.first.compareTo(that.first);
        else
            return this.last.compareTo(that.last);
    }

    @Override
    public boolean equals(Object o)
    {
        if (this == o) return true;
        if (!(o instanceof IPv6AddressRange)) return false;

        IPv6AddressRange that = (IPv6AddressRange) o;

        if (first != null ? !first.equals(that.first) : that.first != null) return false;
        if (last != null ? !last.equals(that.last) : that.last != null) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        int result = first != null ? first.hashCode() : 0;
        result = 31 * result + (last != null ? last.hashCode() : 0);
        return result;
    }

    public IPv6Address getFirst()
    {
        return first;
    }

    public IPv6Address getLast()
    {
        return last;
    }

    /**
     * @see IPv6AddressRange#iterator()
     */
    private final class IPv6AddressRangeIterator implements Iterator<IPv6Address>
    {
        private IPv6Address current = first;

        @Override
        public boolean hasNext()
        {
            return current.compareTo(last) <= 0;
        }

        @Override
        public IPv6Address next()
        {
            if (hasNext())
            {
                IPv6Address result = current;
                current = current.add(1);
                return result;
            }
            else
            {
                throw new NoSuchElementException();
            }
        }

        @Override
        public void remove()
        {
            throw new UnsupportedOperationException("This iterator provides read only access");
        }
    }

    private class IPv6AddressRangeAsSubnetsIterator implements Iterator<IPv6Network>
    {

        private IPv6Address base = first;
        private IPv6Network next;

        @Override
        public IPv6Network next()
        {
            int step;

            if (hasNext())
            {
                step = 0;

                // try setting the step-th bit until we reach a bit that is already set
                while (!(base.setBit(step)).equals(base))
                {
                    // if the max address in this subnet is beyond the end of the range, we went too far
                    if ((base.maximumAddressWithNetworkMask(IPv6NetworkMask.fromPrefixLength(127 - step)).compareTo(last) > 0))
                    {
                        break;
                    }
                    step++;
                }

                // the next subnet is found
                next = IPv6Network.fromAddressAndMask(base, IPv6NetworkMask.fromPrefixLength(128 - step));

                // start the next loop after the end of the subnet just found
                base = next.getLast().add(1);
            }
            else
            {
                throw new NoSuchElementException();
            }

            return next;
        }

        @Override
        public boolean hasNext()
        {
            // there is a next subnet as long as we didn't reach the end of the range
            return (base.compareTo(last) <= 0);
        }

        @Override
        public void remove()
        {
            throw new UnsupportedOperationException("This iterator provides read only access");
        }
    }
}