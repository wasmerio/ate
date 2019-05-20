package com.tokera.examples.common;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.examples.dao.Account;
import com.tokera.examples.dao.MonthlyActivity;

import java.util.Calendar;
import java.util.Date;
import java.util.List;

public class AccountHelper {

    public static MonthlyActivity getCurrentMonthlyActivity(Account acc) {
        AteDelegate d = AteDelegate.get();
        Date now = new Date();

        // Fast path - check the last monthly activity
        if (acc.monthlyActivities.size() > 0) {
            MonthlyActivity last = d.headIO.get(acc.monthlyActivities.get(acc.monthlyActivities.size()-1), MonthlyActivity.class);
            if (now.compareTo(last.start) >= 0 &&
                now.compareTo(last.end) <= 0)
            {
                return last;
            }
        }

        // Slow path - make sure there's no duplicates
        List<MonthlyActivity> monthlyActivities = d.headIO.getMany(acc.monthlyActivities, MonthlyActivity.class);
        for (MonthlyActivity monthlyActivity : monthlyActivities) {
            if (now.compareTo(monthlyActivity.start) >= 0 &&
                now.compareTo(monthlyActivity.end) <= 0)
            {
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

        d.headIO.mergeLater(ret);
        d.headIO.mergeLater(acc);
        return ret;
    }
}
