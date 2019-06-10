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
import java.util.stream.Collectors;

@RequestScoped
@Path("/account/{accountId}")
@PermitReadEntity(name="accountId", clazz= Account.class)
public class AccountREST {
    protected AteDelegate d = AteDelegate.get();
    protected CoinHelper coinHelper = new CoinHelper();
    protected AccountHelper accountHelper = new AccountHelper();

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

        // Carve off the value that was requested
        List<CoinShare> transferShares = coinHelper.carveOfValue(
                d.io.getManyAcrossPartitions(acc.coins, Coin.class).stream().filter(c -> c.type.equals(request.assetType)).collect(Collectors.toList()),
                request.amount);

        // If there's still some remaining then we don't own enough of this asset to meet the desired amount
        BigDecimal transferSharesValue = coinHelper.valueOfShares(transferShares, false);
        if (transferSharesValue.compareTo(request.amount) != 0) {
            throw new WebApplicationException("Insufficient funds [found=" + transferSharesValue + ", needed=" + request.amount + "].", Response.Status.NOT_ACCEPTABLE);
        }

        // Force a merge as the tree structure must be in place before we attempt to immutalize it
        d.io.mergeDeferredAndSync();

        // Claim all the coins
        MessagePrivateKeyDto ownership = d.encryptor.genSignKey();
        Collection<ShareToken> tokens = coinHelper.makeTokens(transferShares, ownership);

        // Force a merge as the tree structure must be in place before we attempt to immutalize it
        d.io.mergeDeferredAndSync();

        // Immutalize the shares that need to be protected
        //coinHelper.immutalizeParentTokens(tokens);

        // Return a transaction token that holds rights to all the shares that the received will be able to take over
        return new TransactionToken(
                tokens,
                "Crediting " + request.amount + " coins of type [" + request.assetType + "]");
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
        List<CoinShare> shares = new ArrayList<CoinShare>();

        for (ShareToken shareToken : transactionToken.getShares()) {
            CoinShare share = d.io.get(shareToken.getShare(), CoinShare.class);
            shares.add(share);

            d.currentRights.impersonateWrite(shareToken.getOwnership());

            share.trustInheritWrite = false;
            share.getTrustAllowWrite().clear();
            d.authorization.authorizeEntityWrite(ownership, share);
            d.io.mergeLater(share);

            if (acc.coins.contains(shareToken.getCoin()) == false) {
                acc.coins.add(shareToken.getCoin());
                d.io.mergeLater(acc);
            }
        }

        // Now write the transaction history
        MonthlyActivity activity = accountHelper.getCurrentMonthlyActivity(acc);
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
        return accountHelper.getCurrentMonthlyActivity(acc);
    }
}