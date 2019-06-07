package com.tokera.examples.rest;

import com.tokera.ate.annotations.PermitReadEntity;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.common.AccountHelper;
import com.tokera.examples.dao.*;
import com.tokera.examples.dto.*;

import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.Response;
import java.math.BigDecimal;
import java.util.*;

@ApplicationScoped
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

        // Find all the shares of the asset that we actually have ownership rights to
        LinkedList<CoinShare> ownedShares = new LinkedList<CoinShare>();
        ownedShares.addAll(d.io.getManyAcrossPartitions(acc.ownerships, CoinShare.class));

        // Create a new ownership key
        MessagePrivateKeyDto ownership = d.encryptor.genSignKey();

        // Now we need to found up a
        List<ShareToken> shareTokens = new ArrayList<ShareToken>();
        BigDecimal remaining = request.amount;
        for (;;) {
            if (ownedShares.isEmpty()) break;
            CoinShare share = ownedShares.removeFirst();

            // Make sure we actually own it and its of the right type of asset
            if (d.authorization.canWrite(share) == false) {
                continue;
            }
            if (d.daoHelper.hasImplicitAuthority(share, request.assetType) == false) {
                continue;
            }

            // If the share is small enough then create a share token so the received can take ownership of it
            if (share.shareAmount.compareTo(remaining) <= 0) {
                shareTokens.add(new ShareToken(share, ownership));
                remaining = remaining.subtract(share.shareAmount);

                // Check if we are done as we have transferred enough of these shares
                if (remaining.compareTo(BigDecimal.ZERO) <= 0) {
                    break;
                }
            }

            // Otherwise we need to split the share in half so that it can be divided
            else
            {
                // We aim to split it in half but if its getting to small just split it by the exact amount needed
                BigDecimal split;
                if (remaining.compareTo(BigDecimal.valueOf(2)) >= 0) {
                    split = share.shareAmount.divide(BigDecimal.valueOf(2));
                    split = BigDecimal.valueOf(Math.max(split.longValue(), 1L));
                } else {
                    split = remaining;
                }

                // We create to child shares and make the original one immutable
                CoinShare left = new CoinShare(share, split);
                CoinShare right = new CoinShare(share, split);
                share.shares.add(left.id);
                share.shares.add(right.id);
                share.trustAllowWrite.clear();
                share.trustInheritWrite = false;
                d.io.mergeLater(share);

                left.trustInheritWrite = false;
                right.trustInheritWrite = false;

                d.authorization.authorizeEntityWrite(ownership, left);
                d.authorization.authorizeEntityWrite(ownership, right);
                d.io.mergeLater(left);
                d.io.mergeLater(right);

                ownedShares.addFirst(left);
                ownedShares.addFirst(right);
            }
        }

        // If there's still some remaining then we don't own enough of this asset to meet the desired amount
        if (remaining.compareTo(BigDecimal.ZERO) < 0) {
            throw new WebApplicationException("Insufficient funds.", Response.Status.NOT_ACCEPTABLE);
        }

        // Return a transaction token that holds rights to all the shares that the received will be able to take over
        return new TransactionToken(shareTokens);
    }

    @POST
    @Path("completeTransaction")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public MonthlyActivity completeTransaction(TransactionToken transactionToken) {
        Account acc = d.io.get(accountId, Account.class);
        MonthlyActivity activity = AccountHelper.getCurrentMonthlyActivity(acc);

        for (ShareToken shareToken : transactionToken.getShares()) {
            CoinShare share = d.io.get(shareToken.getShare(), CoinShare.class);
            d.currentRights.impersonateWrite(shareToken.getOwnership());

            share.trustInheritWrite = false;
            share.getTrustAllowRead().clear();
            d.authorization.authorizeEntity(acc, share);
            d.io.mergeLater(share);

            acc.ownerships.add(share.addressableId());
            d.io.mergeLater(acc);

            TransactionDetails details = new TransactionDetails(activity, share);
            activity.transactions.add(new Transaction(details));
            d.io.mergeLater(details);
            d.io.mergeLater(activity);
        }

        return activity;
    }

    @Path("reconcileTransactions")
    @GET
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    public MonthlyActivity reconcileTransactions() {
        Account acc = d.io.get(accountId, Account.class);
        MonthlyActivity activity = AccountHelper.getCurrentMonthlyActivity(acc);

        // For any assets we don't own anymore then we add a transaction to the new owner
        for (CoinShare share : d.io.getManyAcrossPartitions(acc.ownerships, CoinShare.class)) {
            if (d.authorization.canWrite(share) == false)
            {
                // When we don't own it anymore (as someone else took the rights to it) then we remove it from
                // our ownership list
                acc.ownerships.remove(share.addressableId());
                d.io.mergeLater(acc);

                // Add a transaction details to show that we no longer own it
                TransactionDetails details = new TransactionDetails(activity, share);
                details.amount = details.amount.negate();
                activity.transactions.add(new Transaction(details));
                d.io.mergeLater(details);
                d.io.mergeLater(activity);

                // Add the coin share to the account
                acc.ownerships.add(share.addressableId());
                d.io.mergeLater(acc);
            }
        }

        return activity;
    }

    @GET
    @Path("transactions")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    public MonthlyActivity getTransactions() {
        Account acc = d.io.get(accountId, Account.class);
        return AccountHelper.getCurrentMonthlyActivity(acc);
    }
}