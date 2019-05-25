package com.tokera.examples.dto;

import com.tokera.ate.common.ImmutalizableArrayList;

import java.util.List;

public class TransactionToken {
    public final ImmutalizableArrayList<ShareToken> shares = new ImmutalizableArrayList<ShareToken>();

    public TransactionToken(List<ShareToken> shares) {
        this.shares.addAll(shares);
    }
}