package com.tokera.examples.rest;

import com.tokera.ate.annotations.PermitReadEntity;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.common.AccountHelper;
import com.tokera.examples.dao.*;
import com.tokera.examples.dto.BeginTransactionRequest;
import com.tokera.examples.dto.ShareToken;
import com.tokera.examples.dto.TransactionToken;

import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import java.util.UUID;

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
        MonthlyActivity activity = AccountHelper.getCurrentMonthlyActivity(acc);
        Asset asset = d.headIO.get(request.asset, Asset.class);

        MessagePrivateKeyDto writeRight = d.encryptor.genSignKeyNtru(256);
        MessagePrivateKeyDto readRight = d.encryptor.genEncryptKeyNtru(256);
        ShareToken token = new ShareToken(asset, writeRight, readRight);

        return token;
    }

    @POST
    @Path("completeTransaction")
    @Produces(MediaType.TEXT_PLAIN)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public TransactionDetails completeTransaction(TransactionToken transactionToken) {
        TransactionDetails otherDetails = d.headIO.get(transactionId, TransactionDetails.class);

        Account acc = d.headIO.get(accountId, Account.class);
        MonthlyActivity activity = AccountHelper.getCurrentMonthlyActivity(acc);

        TransactionDetails details = new TransactionDetails(activity, otherDetails.amount.negate(), otherDetails.addressableId(), otherDetails.asset);
        details.details = otherDetails.details;

        Transaction trans = new Transaction(details);
        trans.description = otherDetails.description;

        activity.transactions.add(trans);
        d.headIO.mergeLater(details);
        d.headIO.mergeLater(activity);
        return details;
    }
}