package com.tokera.examples.common;

import com.google.common.collect.Iterators;
import com.tokera.ate.dao.IRoles;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dao.Coin;
import com.tokera.examples.dao.CoinShare;
import com.tokera.examples.dto.ShareToken;

import java.math.BigDecimal;
import java.math.RoundingMode;
import java.util.*;
import java.util.stream.Collectors;

public class CoinHelper {
    protected AteDelegate d = AteDelegate.get();

    /**
     * We aim to split it in half but if its getting to small just split it by the exact amount needed
     */
    public BigDecimal computeDivideAmount(BigDecimal amount, long cnt) {
        if (cnt <= 1) return amount;
        return amount.divide(BigDecimal.valueOf(cnt), 0, RoundingMode.CEILING);
    }

    /**
     * The last share of the amount needs to make sure the amount properly adds up
     */
    public BigDecimal computeLastDivideAmount(BigDecimal amount, long cnt) {
        if (cnt <= 1) return amount;
        BigDecimal divide = computeDivideAmount(amount, cnt);
        BigDecimal total = divide.multiply(BigDecimal.valueOf(cnt-1));
        return amount.subtract(total);
    }

    /**
     * @return Returns the coin that the share has a claim too
     */
    public Coin getCoinFromShare(CoinShare share)
    {
        for (;;) {
            BaseDao parent = d.daoHelper.getParent(share);
            if (parent instanceof CoinShare) {
                share = (CoinShare)parent;
                continue;
            }
            if (parent instanceof Coin) {
                return (Coin)parent;
            }
            throw new RuntimeException("This coin share is not attached to a valid coin.");
        }
    }

    /**
     * Builds a family line all the way to the founding share
     */
    public List<CoinShare> buildFamilyLine(CoinShare share) {
        LinkedList<CoinShare> ret = new LinkedList<>();
        ret.add(share);
        for (; ; ) {
            BaseDao parent = d.daoHelper.getParent(share);
            if (parent instanceof CoinShare) {
                share = (CoinShare) parent;
                ret.addFirst(share);
                continue;
            }
            return ret;
        }
    }

    /**
     * @return Returns the value of this share based on how much its divided up the original coin
     */
    public BigDecimal valueOfShare(CoinShare share) {
        Coin coin = getCoinFromShare(share);
        List<CoinShare> familyLine = buildFamilyLine(share);
        Set<UUID> ids = familyLine.stream().map(s -> s.id).collect(Collectors.toSet());

        BigDecimal value = coin.value;
        for (CoinShare node : familyLine) {
            if (node.shares.size() <= 0) return value;
            if (ids.contains(Iterators.getLast(node.shares.iterator()))) {
                value = computeLastDivideAmount(value, node.shares.size());
            } else {
                value = computeDivideAmount(value, node.shares.size());
            }
        }
        return value;
    }

    /**
     * @return Returns the total value of all the shares supplied
     */
    public BigDecimal valueOfShares(Iterable<CoinShare> shares)
    {
        BigDecimal ret = BigDecimal.ZERO;
        for (CoinShare share : shares) {
            ret = ret.add(valueOfShare(share));
        }
        return ret;
    }

    /**
     * Creates a subcoin the has the same properties as its parent but obviously a slice of its value
     */
    public CoinShare createSubCoin(CoinShare share)
    {
        CoinShare ret = new CoinShare(share);
        share.shares.add(ret.id);
        d.io.mergeLater(share);

        ret.trustInheritWrite = false;
        ret.trustAllowWrite.putAll(share.trustAllowWrite);
        d.io.mergeLater(ret);
        return ret;
    }

    /**
     * Splits a share into smaller parts so that it can be divided up with others
     */
    public Collection<CoinShare> splitShare(CoinShare share)
    {
        if (share.shares.size() > 0) {
            return d.io.getMany(share.shares, CoinShare.class);
        }

        ArrayList<CoinShare> ret = new ArrayList<>();
        ret.add(createSubCoin(share));
        ret.add(createSubCoin(share));
        return  ret;
    }

    /**
     * Turns a bunch of shares of coins into tokens that can be passed to someone else
     */
    public Collection<ShareToken> makeTokens(Iterable<CoinShare> shares, MessagePrivateKeyDto ownership)
    {
        d.currentRights.impersonateWrite(ownership);

        List<ShareToken> tokens = new ArrayList<>();
        for (CoinShare share : shares) {
            share.trustInheritWrite = false;
            share.trustAllowWrite.clear();
            d.authorization.authorizeEntityWrite(ownership, share);

            Coin coin = this.getCoinFromShare(share);
            tokens.add(new ShareToken(coin, share, ownership));
        }
        return tokens;
    }

    /**
     * Finds all the shares of a coins that are owned by the current user
     */
    public List<CoinShare> findOwnedShares(Iterable<Coin> coins) {
        List<CoinShare> ret = new ArrayList<>();

        LinkedList<CoinShare> shares = new LinkedList<>();
        coins.forEach(c -> shares.addAll(d.io.getMany(c.addressableId().partition(), c.shares, CoinShare.class)));
        for (;shares.isEmpty() == false; ) {
            CoinShare share = shares.pop();

            if (d.authorization.canWrite(share) == true) {
                ret.add(share);
            } else {
                shares.addAll(
                    d.io.getMany(
                        share.addressableId().partition(),
                        share.shares,
                        CoinShare.class));
            }
        }
        return ret;
    }

    /**
     * Makes any of the parent shares unclaimable otherwise we could lose the rights to it
     */
    public void immutalizeParentTokens(Iterable<ShareToken> tokens)
    {
        List<CoinShare> shares = new ArrayList<>();
        tokens.forEach(t -> {
            shares.add(d.io.get(t.getShare(), CoinShare.class));
        });
        immutalizeParentShares(shares);
    }

    /**
     * Makes any of the parent shares unclaimable otherwise we could lose the rights to it
     */
    public void immutalizeParentShares(Iterable<CoinShare> shares)
    {
        for (CoinShare share : shares) {
            if (share.parent == null) continue;
            BaseDao parent = d.daoHelper.getParent(share);
            if (parent instanceof IRoles &&
                d.authorization.canWrite(parent))
            {
                IRoles roles = (IRoles)parent;
                roles.getTrustAllowWrite().clear();
                roles.setTrustInheritWrite(false);
                d.io.mergeLater(parent);
            }
        }
    }

    /**
     * Carves off some value from a set of coins and returns it
     */
    public List<CoinShare> carveOfValue(Iterable<Coin> coins, BigDecimal amount) {
        LinkedList<CoinShare> processCoins = new LinkedList<>(this.findOwnedShares(coins));
        LinkedList<CoinShare> transferCoins = new LinkedList<CoinShare>();

        BigDecimal remaining = amount;
        for (;;) {
            if (processCoins.isEmpty()) break;
            CoinShare share = processCoins.removeFirst();

            // If the share is small enough to be forked off then claim it otherwise we need to split the share
            BigDecimal shareValue = this.valueOfShare(share);
            if (shareValue.compareTo(remaining) <= 0) {
                transferCoins.add(share);

                // Check if we are done as we have transferred enough of these shares
                remaining = remaining.subtract(shareValue);
                if (remaining.compareTo(BigDecimal.ZERO) <= 0) {
                    break;
                }
            } else {
                processCoins.addAll(0, splitShare(share));
            }
        }
        return transferCoins;
    }
}
