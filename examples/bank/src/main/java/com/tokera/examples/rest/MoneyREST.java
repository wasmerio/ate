package com.tokera.examples.rest;

import com.google.common.collect.Lists;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import com.tokera.examples.dao.*;
import com.tokera.examples.dto.*;

import javax.enterprise.context.RequestScoped;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.Response;

@RequestScoped
@Path("/money")
public class MoneyREST {
    protected AteDelegate d = AteDelegate.get();

    @POST
    @Path("/print")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public TransactionToken printMoney(CreateAssetRequest request) {
        MessagePublicKeyDto coiningKey = d.implicitSecurity.enquireDomainKey(request.type, true);
        d.genericLogger.info("coining-key: " + coiningKey.getPublicKeyHash());

        Coin coin = new Coin(request.type, request.value);
        d.authorization.authorizeEntityPublicRead(coin);
        d.authorization.authorizeEntityWrite(coiningKey, coin);
        d.io.mergeLater(coin);

        CoinShare coinShare = new CoinShare(coin);
        d.authorization.authorizeEntityWrite(request.ownershipKey, coinShare);
        coin.shares.add(coinShare.id);

        d.io.mergeLater(coin);
        d.io.mergeLater(coinShare);

        //LOG.info(d.yaml.serializeObj(asset));
        //LOG.info(d.yaml.serializeObj(assetShare));

        String description = "Printing " + request.value + " coins of type [" + request.type + "]";
        TransactionToken ret = new TransactionToken(Lists.newArrayList(new ShareToken(coin, coinShare, request.ownershipKey)), description);

        d.io.mergeDeferredAndSync();
        return ret;
    }

    @POST
    @Path("/burn")
    @Produces(MediaType.TEXT_PLAIN)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public boolean burnMoney(RedeemAssetRequest request) {
        for (ShareToken shareToken : request.transactionToken.getShares()) {
            d.currentRights.clearImpersonation();
            d.currentRights.impersonateWrite(shareToken.getOwnership());

            CoinShare coinShare = d.io.get(shareToken.getShare(), CoinShare.class);
            if (d.daoHelper.hasImplicitAuthority(coinShare, request.validateType) == false) {
                throw new WebApplicationException("Asset is not of the correct type.", Response.Status.NOT_ACCEPTABLE);
            }
            coinShare.trustInheritWrite = false;
            coinShare.trustAllowWrite.clear();
            d.io.merge(coinShare);
        }
        return true;
    }
}