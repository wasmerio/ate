package com.tokera.examples.dto;

import java.util.ArrayList;
import java.util.Collection;

public class TransactionToken {
    private String description;
    private ArrayList<ShareToken> shares = new ArrayList<ShareToken>();

    public TransactionToken() {
    }

    public TransactionToken(Collection<ShareToken> shares, String description) {
        this.shares.addAll(shares);
        this.description = description;
    }

    public ArrayList<ShareToken> getShares() {
        return shares;
    }

    public void setShares(ArrayList<ShareToken> shares) {
        this.shares = shares;
    }

    public String getDescription() {
        return description;
    }

    public void setDescription(String description) {
        this.description = description;
    }
}