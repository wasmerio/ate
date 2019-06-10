package com.tokera.examples.common;

import com.google.common.collect.Lists;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.examples.dao.Account;
import com.tokera.examples.dao.Coin;
import com.tokera.examples.dao.CoinShare;
import com.tokera.examples.dao.MonthlyActivity;

import java.math.BigDecimal;
import java.util.*;

public class AccountHelper {
    AteDelegate d = AteDelegate.get();
    protected CoinHelper coinHelper = new CoinHelper();

    /**
     * Computes the balances based on the coins we know about that we have shares in
     */
    public Map<String, BigDecimal> computeBalances(Account acc)
    {
        // Find all the coins for this account
        LinkedList<CoinShare> shares = new LinkedList<>();
        List<Coin> coins = d.io.getManyAcrossPartitions(acc.coins, Coin.class);
        for (Coin coin : coins) {
            List<CoinShare> coinShares = coinHelper.findOwnedShares(Lists.newArrayList(coin));
            if (coinShares.size() < 0) {
                // We have no more shares of this coin so forget about it
                acc.coins.remove(coin.addressableId());
                d.io.mergeLater(acc);
            } else {
                shares.addAll(coinShares);
            }
        }

        // For any assets we don't own anymore then we add a transaction to the new owner
        TreeMap<String, BigDecimal> balances = new TreeMap<String, BigDecimal>();
        for (CoinShare share : shares) {
            Coin coin = coinHelper.getCoinFromShare(share);

            BigDecimal shareValue = coinHelper.valueOfShare(share, true);
            balances.put(coin.type,
                    balances.getOrDefault(coin.type, BigDecimal.ZERO).add(shareValue));
        }
        return balances;
    }

    /**
     * Update the activity balances based on what we computed
     */
    public void reconcileBalance(Account acc, MonthlyActivity activity)
    {
        Map<String, BigDecimal> balances = computeBalances(acc);

        for (String coinType : activity.balances.keySet()) {
            if (balances.containsKey(coinType) == false) {
                activity.balances.remove(coinType);
                d.io.mergeLater(activity);
            }
        }
        for (String coinType : balances.keySet()) {
            BigDecimal left = activity.balances.getOrDefault(coinType, BigDecimal.ZERO);
            BigDecimal right = balances.get(coinType);

            if (left.compareTo(right) != 0) {
                activity.balances.put(coinType, right);
                d.io.mergeLater(activity);
            }
        }
    }

    /**
     * Gets an activity object that represents the current month
     */
    public MonthlyActivity getCurrentMonthlyActivity(Account acc) {
        Date now = new Date();

        // Fast path - check the last monthly activity
        if (acc.monthlyActivities.size() > 0) {
            MonthlyActivity last = d.io.get(acc.monthlyActivities.get(acc.monthlyActivities.size()-1), MonthlyActivity.class);
            if (now.compareTo(last.start) >= 0 &&
                now.compareTo(last.end) <= 0)
            {
                reconcileBalance(acc, last);
                return last;
            }
        }

        // Slow path - make sure there's no duplicates
        List<MonthlyActivity> monthlyActivities = d.io.getMany(acc.monthlyActivities, MonthlyActivity.class);
        for (MonthlyActivity monthlyActivity : monthlyActivities) {
            if (now.compareTo(monthlyActivity.start) >= 0 &&
                now.compareTo(monthlyActivity.end) <= 0)
            {
                reconcileBalance(acc, monthlyActivity);
                return monthlyActivity;
            }
        }

        // Add the entry and return it
        Calendar c = Calendar.getInstance();
        c.set(Calendar.DATE, c.getActualMinimum(Calendar.DAY_OF_MONTH));
        Date start = c.getTime();
        c.add(Calendar.MONTH, 1);
        c.set(Calendar.DATE, c.getActualMinimum(Calendar.DAY_OF_MONTH));
        Date end = c.getTime();

        MonthlyActivity ret = new MonthlyActivity(acc, start, end);
        acc.monthlyActivities.add(ret.id);

        d.io.mergeLater(ret);
        d.io.mergeLater(acc);

        reconcileBalance(acc, ret);
        return ret;
    }
}
