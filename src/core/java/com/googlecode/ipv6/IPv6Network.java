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
import java.util.Iterator;
import java.util.NoSuchElementException;

/**
 * Immutable representation of an IPv6 network based on an address and a prefix length. An IPv6 network is also an IPv6 address range (but
 * not all ranges are valid networks).
 *
 * @author Jan Van Besien
 */
@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public final class IPv6Network extends IPv6AddressRange
{
    public static final IPv6Network MULTICAST_NETWORK = fromString("ff00::/8");

    public static final IPv6Network SITE_LOCAL_NETWORK = fromString("fec0::/48");

    public static final IPv6Network LINK_LOCAL_NETWORK = fromString("fe80::/64");


    private final IPv6Address address;

    private final IPv6NetworkMask networkMask;

    /**
     * Construct from address and network mask.
     *
     * @param address     address
     * @param networkMask network mask
     */
    private IPv6Network(@NonNull IPv6Address address, @NonNull IPv6NetworkMask networkMask)
    {
        super(address.maskWithNetworkMask(networkMask), address.maximumAddressWithNetworkMask(networkMask));

        this.address = address.maskWithNetworkMask(networkMask);
        this.networkMask = networkMask;
    }

    /**
     * Create an IPv6 network from an IPv6Address and an IPv6NetworkMask
     *
     * @param address     IPv6 address (the network address or any other address within the network)
     * @param networkMask IPv6 network mask
     * @return IPv6 network
     */
    public static @NonNull IPv6Network fromAddressAndMask(@NonNull IPv6Address address, @NonNull IPv6NetworkMask networkMask)
    {
        return new IPv6Network(address, networkMask);
    }

    /**
     * Create an IPv6 network from the two addresses within the network. This will construct the smallest possible network ("longest prefix
     * length") which contains both addresses.
     *
     * @param one address one
     * @param two address two, should be bigger than address one
     */
    public static @NonNull IPv6Network fromTwoAddresses(@NonNull IPv6Address one, @NonNull IPv6Address two)
    {
        final IPv6NetworkMask longestPrefixLength = IPv6NetworkMask.fromPrefixLength(IPv6NetworkHelpers.longestPrefixLength(one, two));
        return new IPv6Network(one.maskWithNetworkMask(longestPrefixLength), longestPrefixLength);
    }

    /**
     * Create an IPv6 network from its String representation. For example "1234:5678:abcd:0:0:0:0:0/64" or "2001::ff/128".
     *
     * @param string string representation
     * @return IPv6 network
     */
    public static @NonNull IPv6Network fromString(@NonNull String string)
    {
        if (string.indexOf('/') == -1)
        {
            throw new IllegalArgumentException("Expected format is network-address/prefix-length");
        }

        final String networkAddressString = parseNetworkAddress(string);
        int prefixLength = parsePrefixLength(string);

        final IPv6Address networkAddress = IPv6Address.fromString(networkAddressString);

        return fromAddressAndMask(networkAddress, new IPv6NetworkMask(prefixLength));
    }

    private static @NonNull String parseNetworkAddress(@NonNull String string)
    {
        return string.substring(0, string.indexOf('/'));
    }

    private static int parsePrefixLength(@NonNull String string)
    {
        try
        {
            return Integer.parseInt(string.substring(string.indexOf('/') + 1));
        } catch (NumberFormatException e)
        {
            throw new IllegalArgumentException("Prefix length should be a positive integer");
        }
    }

    /**
     * Split a network in smaller subnets of a given size.
     *
     * @param size size (expressed as {@link com.googlecode.ipv6.IPv6NetworkMask}) of the subnets
     * @return iterator of the splitted subnets.
     * @throws IllegalArgumentException if the requested size is bigger than the original size
     */
    public Iterator<@NonNull IPv6Network> split(@NonNull IPv6NetworkMask size)
    {
        if (size.asPrefixLength() < this.getNetmask().asPrefixLength())
            throw new IllegalArgumentException(String.format("Can not split a network of size %s in subnets of larger size %s",
                                                             this.getNetmask().asPrefixLength(), size.asPrefixLength()));

        return new IPv6NetworkSplitsIterator(size);
    }

    @Override
    public @NonNull String toString()
    {
        return address.toString() + "/" + networkMask.asPrefixLength();
    }

    /**
     * @return like <code>toString</code> but without using shorthand notations for addresses
     */
    public @NonNull String toLongString()
    {
        return address.toLongString() + "/" + networkMask.asPrefixLength();
    }

    @Override
    public boolean equals(Object o)
    {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        if (!super.equals(o)) return false;

        IPv6Network that = (IPv6Network) o;

        if (address != null ? !address.equals(that.address) : that.address != null) return false;
        if (networkMask != null ? !networkMask.equals(that.networkMask) : that.networkMask != null) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        int result = super.hashCode();
        result = 31 * result + (address != null ? address.hashCode() : 0);
        result = 31 * result + (networkMask != null ? networkMask.hashCode() : 0);
        return result;
    }

    public @Nullable IPv6NetworkMask getNetmask()
    {
        return networkMask;
    }

    private final class IPv6NetworkSplitsIterator implements Iterator<@NonNull IPv6Network>
    {
        private final IPv6NetworkMask size;

        private IPv6Network current;

        private BigInteger nbrAddressesPerSplit;

        public IPv6NetworkSplitsIterator(@NonNull IPv6NetworkMask size)
        {
            this.size = size;
            this.nbrAddressesPerSplit = BigInteger.ONE.shiftLeft(128 - size.asPrefixLength());
            this.current = IPv6Network.fromAddressAndMask(IPv6Network.this.address, size);
        }

        @Override
        public boolean hasNext()
        {
            return current.getLast().compareTo(IPv6Network.this.getLast()) <= 0;
        }

        @Override
        public @NonNull IPv6Network next()
        {
            if (hasNext())
            {
                IPv6Network result = current;
                current = calculateNext(current);
                return result;
            }
            else
            {
                throw new NoSuchElementException();
            }
        }

        private @NonNull IPv6Network calculateNext(@NonNull IPv6Network current)
        {
            BigInteger next = current.address.toBigInteger().add(nbrAddressesPerSplit);
            return IPv6Network.fromAddressAndMask(IPv6Address.fromBigInteger(next), size);
        }

        @Override
        public void remove()
        {
            throw new UnsupportedOperationException("This iterator provides read only access");
        }
    }
}