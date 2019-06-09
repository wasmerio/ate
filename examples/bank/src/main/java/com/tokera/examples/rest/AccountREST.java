package com.tokera.examples.rest;

import com.tokera.ate.annotations.PermitReadEntity;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.common.AccountHelper;
import com.tokera.examples.common.CoinHelper;
import com.tokera.examples.dao.*;
import com.tokera.examples.dto.*;

import javax.enterprise.context.RequestScoped;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.Response;
import java.math.BigDecimal;
import java.util.*;

@RequestScoped
@Path("/account/{accountId}")
@PermitReadEntity(name="accountId", clazz= Account.class)
public class AccountREST {
    protected AteDelegate d = AteDelegate.get();

    @SuppressWarnings("initialization.fields.uninitialized")
    @PathParam("accountId")
    protected UUID accountId;

    @POST
    @Path("beginTransaction")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public TransactionToken beginTransaction(BeginTransactionRequest request) {
        Account acc = d.io.get(accountId, Account.class);
        d.currentRights.impersonate(acc);

        // Find all the shares of the asset that we actually have ownership rights to
        LinkedList<CoinShare> ownedShares = new LinkedList<CoinShare>();
        ownedShares.addAll(d.io.getManyAcrossPartitions(acc.ownerships, CoinShare.class));

        // Create a new ownership key
        MessagePrivateKeyDto ownership = d.encryptor.genSignKey();
        d.currentRights.impersonateWrite(ownership);

        // Now we need to found up a
        LinkedList<CoinShare> lostShared = new LinkedList<CoinShare>();
        List<ShareToken> shareTokens = new ArrayList<ShareToken>();
        BigDecimal remaining = request.amount;
        for (;;) {
            if (ownedShares.isEmpty()) break;
            CoinShare share = ownedShares.removeFirst();

            // Make sure we actually own it and its of the right type of asset
            if (d.daoHelper.hasImplicitAuthority(share, request.assetType) == false) {
                continue;
            }
            if (d.authorization.canWrite(share) == false) {
                continue;
            }

            // If the share is too big then we need to split it up
            if (share.shareAmount.compareTo(remaining) > 0)
            {
                // We aim to split it in half but if its getting to small just split it by the exact amount needed
                BigDecimal split;
                if (remaining.compareTo(BigDecimal.valueOf(2)) >= 0) {
                    split = share.shareAmount.divide(BigDecimal.valueOf(2));
                    split = BigDecimal.valueOf(Math.max(split.longValue(), 1L));
                } else {
                    split = remaining;
                }

                // Split the coin up based on the divider
                for (CoinShare child : CoinHelper.splitCoin(share, split)) {
                    ownedShares.addFirst(child);
                }
            }

            // Otherwise we claim it
            else
            {
                shareTokens.add(new ShareToken(share, ownership));
                lostShared.add(share);
                share.trustInheritWrite = false;
                share.trustAllowWrite.clear();
                d.authorization.authorizeEntityWrite(acc, share);
                d.authorization.authorizeEntityWrite(ownership, share);

                // Check if we are done as we have transferred enough of these shares
                remaining = remaining.subtract(share.shareAmount);
                if (remaining.compareTo(BigDecimal.ZERO) <= 0) {
                    break;
                }
            }
        }

        // If there's still some remaining then we don't own enough of this asset to meet the desired amount
        if (remaining.compareTo(BigDecimal.ZERO) > 0) {
            throw new WebApplicationException("Insufficient funds.", Response.Status.NOT_ACCEPTABLE);
        }

        // Force a merge
        d.io.mergeDeferred();
        d.io.sync();

        // Now write the transaction history
        String description = "Debit of " + request.amount + " coins of type [" + request.assetType + "].";
        MonthlyActivity activity = AccountHelper.getCurrentMonthlyActivity(acc);
        TransactionDetails details = new TransactionDetails(activity, lostShared, description);
        activity.transactions.add(new Transaction(details));
        d.io.mergeLater(details);
        d.io.mergeLater(activity);

        // Return a transaction token that holds rights to all the shares that the received will be able to take over
        description = "Crediting " + request.amount + " coins of type [" + request.assetType + "]";
        return new TransactionToken(shareTokens, description);
    }

    @POST
    @Path("completeTransaction")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public MonthlyActivity completeTransaction(TransactionToken transactionToken) {
        Account acc = d.io.get(accountId, Account.class);
        d.currentRights.impersonate(acc);
        MessagePrivateKeyDto ownership = d.authorization.getOrCreateImplicitRightToWrite(acc);

        // Prepare aggregate counters
        BigDecimal amount = BigDecimal.ZERO;
        List<CoinShare> shares = new ArrayList<CoinShare>();

        for (ShareToken shareToken : transactionToken.getShares()) {
            CoinShare share = d.io.get(shareToken.getShare(), CoinShare.class);
            shares.add(share);
            amount = amount.add(share.shareAmount);

            d.currentRights.impersonateWrite(shareToken.getOwnership());

            share.trustInheritWrite = false;
            share.getTrustAllowWrite().clear();
            d.authorization.authorizeEntityWrite(ownership, share);
            d.io.mergeLater(share);

            acc.ownerships.add(share.addressableId());
            d.io.mergeLater(acc);
        }

        // Now write the transaction history
        MonthlyActivity activity = AccountHelper.getCurrentMonthlyActivity(acc);
        TransactionDetails details = new TransactionDetails(activity, shares, transactionToken.getDescription());
        activity.transactions.add(new Transaction(details));
        d.io.mergeLater(details);
        d.io.mergeLater(activity);

        // Force a merge
        d.io.mergeDeferred();
        d.io.sync();
        return activity;
    }

    @GET
    @Path("transactions")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    public MonthlyActivity getTransactions() {
        Account acc = d.io.get(accountId, Account.class);
        d.currentRights.impersonate(acc);
        return AccountHelper.getCurrentMonthlyActivity(acc);
    }
}