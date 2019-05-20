package com.tokera.examples.dto;

import com.tokera.ate.common.ImmutalizableArrayList;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dao.Share;

import java.math.BigDecimal;
import java.util.List;

public class TransactionToken {
    public final ImmutalizableArrayList<ShareToken> shares = new ImmutalizableArrayList<ShareToken>();

    public TransactionToken(List<ShareToken> shares) {
        this.shares.addAll(shares);
    }
}