package com.tokera.examples.rest;

import com.tokera.ate.annotations.PermitReadEntity;
import com.tokera.ate.delegates.AteDelegate;
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
        Account acc = d.headIO.get(accountId, Account.class);

        // Find all the shares of the asset that we actually have ownership rights to
        LinkedList<AssetShare> ownedShares = new LinkedList<AssetShare>();
        ownedShares.addAll(d.headIO.getManyAcrossPartitions(acc.ownerships, AssetShare.class));

        // Now we need to found up a
        List<ShareToken> shareTokens = new ArrayList<ShareToken>();
        BigDecimal remaining = request.amount;
        for (;;) {
            AssetShare share = ownedShares.removeFirst();
            if (share == null) break;

            // Make sure we actually own it and its of the right type of asset
            if (d.authorization.canWrite(share) == false) {
                continue;
            }
            if (d.daoHelper.hasImplicitAuthority(share, request.assetType) == false) {
                continue;
            }

            // If the share is small enough then create a share token so the received can take ownership of it
            if (share.shareAmount.compareTo(remaining) <= 0) {
                shareTokens.add(new ShareToken(share));
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
                AssetShare left = new AssetShare(share, split);
                AssetShare right = new AssetShare(share, split);
                share.shares.add(left.id);
                share.shares.add(right.id);
                share.rightsWrite.clear();
                share.trustInheritWrite = false;
                d.headIO.mergeLater(share);

                ownedShares.addFirst(left);
                ownedShares.addFirst(right);
            }
        }

        // If there's still some remaining then we don't own enough of this asset to meet the desired amount
        if (remaining.compareTo(BigDecimal.ZERO) != 0) {
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
        Account acc = d.headIO.get(accountId, Account.class);
        MonthlyActivity activity = AccountHelper.getCurrentMonthlyActivity(acc);

        for (ShareToken shareToken : transactionToken.getShares()) {
            AssetShare share = d.headIO.get(shareToken.getShare(), AssetShare.class);

            share.trustInheritWrite = false;
            share.getTrustAllowRead().clear();
            d.authorization.authorizeEntity(acc, share);
            d.headIO.mergeLater(share);

            acc.ownerships.add(share.addressableId());
            d.headIO.mergeLater(acc);

            TransactionDetails details = new TransactionDetails(activity, share);
            activity.transactions.add(new Transaction(details));
            d.headIO.mergeLater(details);
            d.headIO.mergeLater(activity);
        }

        return activity;
    }

    @Path("reconcileTransactions")
    @GET
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    public MonthlyActivity reconcileTransactions() {
        Account acc = d.headIO.get(accountId, Account.class);
        MonthlyActivity activity = AccountHelper.getCurrentMonthlyActivity(acc);

        // For any assets we don't own anymore then we add a transaction to the new owner
        for (AssetShare share : d.headIO.getManyAcrossPartitions(acc.ownerships, AssetShare.class)) {
            if (d.authorization.canWrite(share) == false)
            {
                // When we don't own it anymore (as someone else took the rights to it) then we remove it from
                // our ownership list
                acc.ownerships.remove(share.addressableId());
                d.headIO.mergeLater(acc);

                // Add a transaction details to show that we no longer own it
                TransactionDetails details = new TransactionDetails(activity, share);
                details.amount = details.amount.negate();
                activity.transactions.add(new Transaction(details));
                d.headIO.mergeLater(details);
                d.headIO.mergeLater(activity);
            }
        }

        return activity;
    }
}