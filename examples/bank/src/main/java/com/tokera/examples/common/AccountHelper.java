package com.tokera.examples.common;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.examples.dao.Account;
import com.tokera.examples.dao.CoinShare;
import com.tokera.examples.dao.MonthlyActivity;

import java.math.BigDecimal;
import java.util.*;

public class AccountHelper {

    public static void reconcileBalance(Account acc, MonthlyActivity activity) {
        AteDelegate d = AteDelegate.get();

        // For any assets we don't own anymore then we add a transaction to the new owner
        TreeMap<String, BigDecimal> balances = new TreeMap<String, BigDecimal>();
        LinkedList<CoinShare> shares = new LinkedList<CoinShare>();
        shares.addAll(d.io.getManyAcrossPartitions(acc.ownerships, CoinShare.class));

        for (;;) {
            if (shares.isEmpty()) break;
            CoinShare share = shares.poll();
            PUUID shareId = share.addressableId();

            if (d.authorization.canWrite(share) == true)
            {
                balances.put(share.type,
                             balances.getOrDefault(share.type, BigDecimal.ZERO).add(share.shareAmount));

                if (acc.ownerships.contains(shareId) == false) {
                    acc.ownerships.add(shareId);
                    d.io.mergeLater(acc);
                }
            } else  {
                if (acc.ownerships.contains(shareId) == true) {
                    acc.ownerships.remove(shareId);
                    d.io.mergeLater(acc);
                }

                // If we own any of the children then add them
                shares.addAll(d.io.getMany(share.addressableId().partition(), share.shares, CoinShare.class));
            }
        }

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

    public static MonthlyActivity getCurrentMonthlyActivity(Account acc) {
        AteDelegate d = AteDelegate.get();
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
