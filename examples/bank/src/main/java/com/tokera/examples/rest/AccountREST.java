package com.tokera.examples.rest;

import com.tokera.ate.annotations.PermitReadEntity;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
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
        Account acc = d.io.read(accountId, Account.class);
        d.currentRights.impersonate(acc);

        List<CoinShare> transferShares = d.io.underTransaction(true, () -> {
            // Carve off the value that was requested
            List<CoinShare> shares = coinHelper.carveOfValue(
                    d.io.read(acc.coins, Coin.class).stream().filter(c -> c.type.equals(request.assetType)).collect(Collectors.toList()),
                    request.amount);

            // If there's still some remaining then we don't own enough of this asset to meet the desired amount
            BigDecimal transferSharesValue = coinHelper.valueOfShares(shares, false);
            if (transferSharesValue.compareTo(request.amount) != 0) {
                throw new WebApplicationException("Insufficient funds [found=" + transferSharesValue + ", needed=" + request.amount + "].", Response.Status.NOT_ACCEPTABLE);
            }
            return shares;
        });


        Collection<ShareToken> tokens = d.io.underTransaction(true, () -> {
            // Claim all the coins
            PrivateKeyWithSeedDto ownership = d.encryptor.genSignKeyAndSeed();
            return coinHelper.makeTokens(transferShares, ownership);
        });

        // Immutalize the shares that need to be protected
        coinHelper.immutalizeParentTokens(tokens);

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
        return d.io.underTransaction(true, () -> {
            Account acc = d.io.read(accountId, Account.class);
            d.currentRights.impersonate(acc);
            PrivateKeyWithSeedDto ownership = d.authorization.getOrCreateImplicitRightToWrite(acc);

            // Prepare aggregate counters
            List<CoinShare> shares = new ArrayList<CoinShare>();

            for (ShareToken shareToken : transactionToken.getShares()) {
                CoinShare share = d.io.read(shareToken.getShare(), CoinShare.class);
                shares.add(share);

                d.currentRights.impersonateWrite(shareToken.getOwnership());

                share.trustInheritWrite = false;
                share.getTrustAllowWrite().clear();
                d.authorization.authorizeEntityWrite(ownership, share);
                d.io.write(share);

                if (acc.coins.contains(shareToken.getCoin()) == false) {
                    acc.coins.add(shareToken.getCoin());
                    d.io.write(acc);
                }
            }

            // Now write the transaction history
            MonthlyActivity activity = accountHelper.getCurrentMonthlyActivity(acc);
            TransactionDetails details = new TransactionDetails(activity, shares, transactionToken.getDescription());
            activity.transactions.add(new Transaction(details));
            d.io.write(details);
            d.io.write(activity);

            return activity;
        });
    }

    @GET
    @Path("transactions")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    public MonthlyActivity getTransactions() {
        Account acc = d.io.read(accountId, Account.class);
        d.currentRights.impersonate(acc);
        return accountHelper.getCurrentMonthlyActivity(acc);
    }
}