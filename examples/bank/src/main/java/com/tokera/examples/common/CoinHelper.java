package com.tokera.examples.common;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.examples.dao.CoinShare;

import java.math.BigDecimal;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;

public class CoinHelper {

    public static CoinShare createSubCoin(CoinShare share, BigDecimal amount) {
        AteDelegate d = AteDelegate.get();

        CoinShare ret = new CoinShare(share, amount);
        share.shares.add(ret.id);
        d.io.mergeLater(share);

        ret.trustInheritWrite = false;
        ret.trustAllowWrite.putAll(share.trustAllowWrite);
        d.io.mergeLater(ret);
        return ret;
    }

    public static Collection<CoinShare> splitCoin(CoinShare share, BigDecimal divider) {
        AteDelegate d = AteDelegate.get();

        // Perform some checks on the divider
        if (divider.compareTo(BigDecimal.ZERO) <= 0 ||
            divider.compareTo(share.shareAmount) >= 0) {
            throw new RuntimeException("You must split the coin somewhere in its middle.");
        }

        // Compute the amounts
        BigDecimal amtLeft = divider;
        BigDecimal amtRight = share.shareAmount.subtract(divider);

        // Create the subcoins
        List<CoinShare> subShares = new ArrayList<CoinShare>();
        subShares.add(createSubCoin(share, amtLeft));
        subShares.add(createSubCoin(share, amtRight));

        // Make this coin immutable
        share.trustAllowWrite.clear();
        share.trustInheritWrite = false;
        d.io.mergeLater(share);

        // Now return the child coins to the caller
        return subShares;
    }
}
