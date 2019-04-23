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

import java.util.*;

/**
 * Immutable representation of an IPv6 address pool.
 * <p/>
 * An IPv6 address pool is like an IPv6 address range in which some addresses are "free" and some are "allocated". Think "dhcp server".
 * Addresses are allocated in whole subnet blocks at once. These subnet blocks have a predefined prefix length for the whole allocatable
 * range.
 *
 * @author Jan Van Besien
 */
@DefaultQualifier(Nullable.class)
@SuppressWarnings({"argument.type.incompatible", "return.type.incompatible", "dereference.of.nullable", "iterating.over.nullable", "method.invocation.invalid", "override.return.invalid", "unnecessary.equals", "known.nonnull", "flowexpr.parse.error.postcondition", "unboxing.of.nullable", "accessing.nullable", "type.invalid.annotations.on.use", "switching.nullable", "initialization.fields.uninitialized"})
public final class IPv6AddressPool
{
    private final IPv6AddressRange underlyingRange;

    private final SortedSet<IPv6AddressRange> freeRanges;

    private final IPv6NetworkMask allocationSubnetSize;

    private final IPv6Network lastAllocated;

    /**
     * Create a pool of the given range (boundaries inclusive) which is completely free. The given subnet size is the network mask (thus
     * size) of the allocated subnets in this range. This constructor verifies that the whole range is "aligned" with subnets of this size
     * (i.e. there should not be a waste of space in the beginning or end which is smaller than one subnet of the given subnet size).
     *
     * @param range                range from within to allocate
     * @param allocationSubnetSize size of the subnets that will be allocated
     */
    public static IPv6AddressPool fromRangeAndSubnet(final IPv6AddressRange range,
                                                     final IPv6NetworkMask allocationSubnetSize)
    {
        // in the beginning, all is free
        return new IPv6AddressPool(range, allocationSubnetSize, new TreeSet<IPv6AddressRange>(Arrays.asList(range)), null);
    }

    /**
     * Private constructor to construct a pool with a given set of free ranges and a network which was just allocated.
     *
     * @param range                range from within to allocate
     * @param allocationSubnetSize size of the subnets that will be allocated
     * @param freeRanges           free ranges in the allocatable IP address range
     */
    private IPv6AddressPool(final IPv6AddressRange range, final IPv6NetworkMask allocationSubnetSize,
                            final SortedSet<IPv6AddressRange> freeRanges, final IPv6Network lastAllocated)
    {
        this.underlyingRange = range;

        this.allocationSubnetSize = allocationSubnetSize;
        this.freeRanges = Collections.unmodifiableSortedSet(freeRanges);
        this.lastAllocated = lastAllocated;

        validateFreeRanges(underlyingRange, freeRanges);
        validateRangeIsMultipleOfSubnetsOfGivenSize(underlyingRange, allocationSubnetSize);
    }

    private void validateFreeRanges(IPv6AddressRange range, SortedSet<IPv6AddressRange> toValidate)
    {
        if (!toValidate.isEmpty() && !checkWithinBounds(range, toValidate))
            throw new IllegalArgumentException("invalid free ranges: not all within bounds of overall range");

        // TODO: some more validations would be useful. For example the free ranges should be defragmented and non overlapping etc
    }

    private boolean checkWithinBounds(IPv6AddressRange range, SortedSet<IPv6AddressRange> toValidate)
    {
        return (toValidate.first().getFirst().compareTo(range.getFirst()) >= 0
                && toValidate.last().getLast().compareTo(range.getLast()) <= 0);
    }

    private void validateRangeIsMultipleOfSubnetsOfGivenSize(IPv6AddressRange range, IPv6NetworkMask allocationSubnetSize)
    {
        final int allocatableBits = 128 - allocationSubnetSize.asPrefixLength();

        if (range.getFirst().numberOfTrailingZeroes() < allocatableBits)
            throw new IllegalArgumentException(
                    "range [" + this + "] is not aligned with prefix length [" + allocationSubnetSize.asPrefixLength() + "], "
                            + "first address should end with " +
                            allocatableBits + " zero bits");

        if (range.getLast().numberOfTrailingOnes() < allocatableBits)
            throw new IllegalArgumentException(
                    "range [" + this + "] is not aligned with prefix length [" + allocationSubnetSize.asPrefixLength()
                            + "], last address should end with " +
                            allocatableBits + " one bits");
    }

    /**
     * @return the last IPv6Network which was allocated or null if none was allocated yet
     */
    public IPv6Network getLastAllocated()
    {
        return lastAllocated;
    }

    /**
     * Allocate the first available subnet from the pool.
     *
     * @return resulting pool
     */
    public IPv6AddressPool allocate()
    {
        if (!isExhausted())
        {
            // get the first range of free subnets, and take the first subnet of that range
            final IPv6AddressRange firstFreeRange = freeRanges.first();
            final IPv6Network allocated = IPv6Network.fromAddressAndMask(firstFreeRange.getFirst(), allocationSubnetSize);

            return doAllocate(allocated, firstFreeRange);
        }
        else
        {
            // exhausted
            return null;
        }
    }

    /**
     * Allocate the given subnet from the pool.
     *
     * @param toAllocate subnet to allocate from the pool
     * @return resulting pool
     */
    public IPv6AddressPool allocate(IPv6Network toAllocate)
    {
        if (!contains(toAllocate))
            throw new IllegalArgumentException(
                    "can not allocate network which is not contained in the pool to allocate from [" + toAllocate + "]");

        if (!this.allocationSubnetSize.equals(toAllocate.getNetmask()))
            throw new IllegalArgumentException("can not allocate network with prefix length /" + toAllocate.getNetmask().asPrefixLength() +
                                                       " from a pool configured to hand out subnets with prefix length /"
                                                       + allocationSubnetSize);

        // go find the range that contains the requested subnet
        final IPv6AddressRange rangeToAllocateFrom = findFreeRangeContaining(toAllocate);

        if (rangeToAllocateFrom != null)
        {
            // found a range in which this subnet is free, allocate it
            return doAllocate(toAllocate, rangeToAllocateFrom);
        }
        else
        {
            // requested subnet not free
            return null;
        }
    }

    private IPv6AddressRange findFreeRangeContaining(IPv6Network toAllocate)
    {
        // split around the subnet to allocate
        final SortedSet<IPv6AddressRange> head = freeRanges.headSet(toAllocate);
        final SortedSet<IPv6AddressRange> tail = freeRanges.tailSet(toAllocate);

        // the range containing the network to allocate is either the first of the tail, or the last of the head, or it doesn't exist
        if (!head.isEmpty() && head.last().contains(toAllocate))
        {
            return head.last();
        }
        else if (!tail.isEmpty() && tail.first().contains(toAllocate))
        {
            return tail.first();
        }
        else
        {
            return null;
        }
    }

    /**
     * Private helper method to perform the allocation of a subnet within one of the free ranges.
     *
     * @param toAllocate          subnet to allocate
     * @param rangeToAllocateFrom free range to allocate from
     * @return resulting pool
     */
    private IPv6AddressPool doAllocate(final IPv6Network toAllocate, final IPv6AddressRange rangeToAllocateFrom)
    {
        assert freeRanges.contains(rangeToAllocateFrom);
        assert rangeToAllocateFrom.contains(toAllocate);

        final TreeSet<IPv6AddressRange> newFreeRanges = new TreeSet<IPv6AddressRange>(this.freeRanges);

        // remove range from free ranges
        newFreeRanges.remove(rangeToAllocateFrom);

        // from the range, remove the allocated subnet
        final List<IPv6AddressRange> newRanges = rangeToAllocateFrom.remove(toAllocate);

        // and add the resulting ranges as new free ranges
        newFreeRanges.addAll(newRanges);

        return new IPv6AddressPool(underlyingRange, allocationSubnetSize, newFreeRanges, toAllocate);
    }

    /**
     * Give a network back to the pool (de-allocate).
     *
     * @param toDeAllocate network to de-allocate
     */
    public IPv6AddressPool deAllocate(final IPv6Network toDeAllocate)
    {
        if (!contains(toDeAllocate))
        {
            throw new IllegalArgumentException(
                    "Network to de-allocate[" + toDeAllocate + "] is not contained in this allocatable range [" + this + "]");
        }

        // find ranges just in front or after the network to deallocate. These are the ranges to mergeThreeWay with to prevent fragmentation.
        final IPv6AddressRange freeRangeBeforeNetwork = findFreeRangeBefore(toDeAllocate);
        final IPv6AddressRange freeRangeAfterNetwork = findFreeRangeAfter(toDeAllocate);

        final TreeSet<IPv6AddressRange> newFreeRanges = new TreeSet<IPv6AddressRange>(this.freeRanges);

        if ((freeRangeBeforeNetwork == null) && (freeRangeAfterNetwork == null))
        {
            // nothing to "defragment"
            newFreeRanges.add(toDeAllocate);
        }
        else
        {
            if ((freeRangeBeforeNetwork != null) && (freeRangeAfterNetwork != null))
            {
                // mergeThreeWay two existing ranges
                newFreeRanges.remove(freeRangeBeforeNetwork);
                newFreeRanges.remove(freeRangeAfterNetwork);
                newFreeRanges.add(IPv6AddressRange.fromFirstAndLast(freeRangeBeforeNetwork.getFirst(), freeRangeAfterNetwork.getLast()));
            }
            else if (freeRangeBeforeNetwork != null)
            {
                // append
                newFreeRanges.remove(freeRangeBeforeNetwork);
                newFreeRanges.add(IPv6AddressRange.fromFirstAndLast(freeRangeBeforeNetwork.getFirst(), toDeAllocate.getLast()));
            }
            else /*if (freeRangeAfterNetwork != null)*/
            {
                // prepend
                newFreeRanges.remove(freeRangeAfterNetwork);
                newFreeRanges.add(IPv6AddressRange.fromFirstAndLast(toDeAllocate.getFirst(), freeRangeAfterNetwork.getLast()));
            }
        }

        return new IPv6AddressPool(underlyingRange, allocationSubnetSize, newFreeRanges, getLastAllocated());
    }

    /**
     * Private helper method to find the free range just before the given network.
     */
    private IPv6AddressRange findFreeRangeBefore(IPv6Network network)
    {
        for (IPv6AddressRange freeRange : freeRanges)
        {
            if (freeRange.getLast().add(1).equals(network.getFirst()))
            {
                return freeRange;
            }
        }

        // not found
        return null;
    }

    /**
     * Private helper method to find the free range just after the given address.
     */
    private IPv6AddressRange findFreeRangeAfter(IPv6Network network)
    {
        for (IPv6AddressRange freeRange : freeRanges)
        {
            if (freeRange.getFirst().subtract(1).equals(network.getLast()))
            {
                return freeRange;
            }
        }

        // not found
        return null;
    }

    /**
     * @return true if no subnets are free in this pool, false otherwize
     */
    public boolean isExhausted()
    {
        return freeRanges.isEmpty();
    }

    public boolean isFree(final IPv6Network network)
    {
        if (network == null)
            throw new IllegalArgumentException("network invalid [null]");

        if (!this.allocationSubnetSize.equals(network.getNetmask()))
            throw new IllegalArgumentException(
                    "network of prefix length [" + network.getNetmask().asPrefixLength()
                            + "] can not be free in a pool which uses prefix length [" +
                            allocationSubnetSize + "]");

        // find a free range that contains the network
        for (IPv6AddressRange freeRange : freeRanges)
        {
            if (freeRange.contains(network))
            {
                return true;
            }
        }

        // nothing found
        return false;
    }

    /**
     * @return all networks (all with the same fixed prefix length) which are free in this pool
     */
    public Iterable<IPv6Network> freeNetworks()
    {
        return new Iterable<IPv6Network>()
        {
            @Override
            public Iterator<IPv6Network> iterator()
            {
                return new Iterator<IPv6Network>()
                {
                    /*
                     * Iteration is implemented by allocating from a separate pool.
                     */

                    private IPv6AddressPool poolInstanceUsedForIteration = IPv6AddressPool.this;

                    @Override
                    public boolean hasNext()
                    {
                        return !poolInstanceUsedForIteration.isExhausted();
                    }

                    @Override
                    public IPv6Network next()
                    {
                        if (hasNext())
                        {
                            poolInstanceUsedForIteration = poolInstanceUsedForIteration.allocate();
                            return poolInstanceUsedForIteration.lastAllocated;
                        }
                        else
                        {
                            throw new NoSuchElementException();
                        }
                    }

                    @Override
                    public void remove()
                    {
                        throw new UnsupportedOperationException("remove not supported");
                    }
                };
            }
        };
    }

//    /**
//     * @return all networks (all with the same fixed prefix length) which are allocated in this pool
//     */
//    public Iterable<IPv6Network> allocatedNetworks()
//    {
//        return new Iterable<IPv6Network>()
//        {
//            @Override
//            public Iterator<IPv6Network> iterator()
//            {
//                return new Iterator<IPv6Network>()
//                {
//                    @Override
//                    public boolean hasNext()
//                    {
//                        throw new UnsupportedOperationException("TODO: implement hasNext");
//                    }
//
//                    @Override
//                    public IPv6Network next()
//                    {
//                        throw new UnsupportedOperationException("TODO: implement next");
//                    }
//
//                    @Override
//                    public void remove()
//                    {
//                        throw new UnsupportedOperationException("TODO: implement remove");
//                    }
//                };
//            }
//        };
//    }

    @Override
    public boolean equals(Object o)
    {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;

        IPv6AddressPool that = (IPv6AddressPool) o;

        if (allocationSubnetSize != null ? !allocationSubnetSize.equals(that.allocationSubnetSize) : that.allocationSubnetSize != null)
            return false;
        if (freeRanges != null ? !freeRanges.equals(that.freeRanges) : that.freeRanges != null) return false;
        if (lastAllocated != null ? !lastAllocated.equals(that.lastAllocated) : that.lastAllocated != null) return false;
        if (underlyingRange != null ? !underlyingRange.equals(that.underlyingRange) : that.underlyingRange != null) return false;

        return true;
    }

    @Override
    public int hashCode()
    {
        int result = underlyingRange != null ? underlyingRange.hashCode() : 0;
        result = 31 * result + (freeRanges != null ? freeRanges.hashCode() : 0);
        result = 31 * result + (allocationSubnetSize != null ? allocationSubnetSize.hashCode() : 0);
        result = 31 * result + (lastAllocated != null ? lastAllocated.hashCode() : 0);
        return result;
    }


    // delegation methods

    public boolean contains(IPv6Address address)
    {
        return underlyingRange.contains(address);
    }

    public boolean contains(IPv6AddressRange range)
    {
        return underlyingRange.contains(range);
    }

    public boolean overlaps(IPv6AddressRange range)
    {
        return underlyingRange.overlaps(range);
    }

    public IPv6Address getFirst()
    {
        return underlyingRange.getFirst();
    }

    public IPv6Address getLast()
    {
        return underlyingRange.getLast();
    }

    @Override
    public String toString()
    {
        return underlyingRange.toString();
    }

    /**
     * @return like <code>toString</code> but without using shorthand notations for addresses
     */
    public String toLongString()
    {
        return underlyingRange.toLongString();
    }

}