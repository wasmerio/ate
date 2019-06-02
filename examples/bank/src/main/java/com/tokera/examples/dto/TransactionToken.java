package com.tokera.examples.dto;

import java.util.ArrayList;
import java.util.Collection;

public class TransactionToken {
    private ArrayList<ShareToken> shares = new ArrayList<ShareToken>();

    public TransactionToken() {
    }

    public TransactionToken(Collection<ShareToken> shares) {
        this.shares.addAll(shares);
    }

    public ArrayList<ShareToken> getShares() {
        return shares;
    }

    public void setShares(ArrayList<ShareToken> shares) {
        this.shares = shares;
    }
}