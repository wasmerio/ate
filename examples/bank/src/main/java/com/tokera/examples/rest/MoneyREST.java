package com.tokera.examples.rest;

import com.tokera.ate.annotations.PermitReadEntity;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.examples.dao.*;
import com.tokera.examples.dto.CreateAssetRequest;
import com.tokera.examples.dto.RedeemAssetRequest;
import com.tokera.examples.dto.ShareToken;
import com.tokera.examples.dto.TransactionToken;
import edu.emory.mathcs.backport.java.util.Collections;

import javax.enterprise.context.ApplicationScoped;
import javax.ws.rs.*;
import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.Response;

@ApplicationScoped
@Path("/money")
@PermitReadEntity(name="accountId", clazz= Account.class)
public class MoneyREST {
    protected AteDelegate d = AteDelegate.get();

    @POST
    @Path("/print")
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public TransactionToken printMoney(CreateAssetRequest request) {
        Asset asset = new Asset(request.type, request.value);
        d.authorization.authorizeEntityPublicRead(asset);

        AssetShare assetShare = new AssetShare(asset, request.value);
        assetShare.trustInheritRead = true;
        assetShare.trustInheritWrite = false;
        d.authorization.authorizeEntity(assetShare, assetShare);
        asset.shares.add(assetShare.id);

        d.headIO.mergeLater(asset);
        d.headIO.mergeLater(assetShare);
        return new TransactionToken(Collections.singletonList(new ShareToken(assetShare)));
    }

    @POST
    @Path("/burn")
    @Produces(MediaType.TEXT_PLAIN)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public boolean burnMoney(RedeemAssetRequest request) {
        AssetShare assetShare = d.headIO.get(request.shareToken.share, AssetShare.class);
        if (d.daoHelper.hasImplicitAuthority(assetShare, request.validateType) == false) {
            throw new WebApplicationException("Asset is not of the correct type.", Response.Status.NOT_ACCEPTABLE);
        }
        assetShare.trustInheritWrite = false;
        assetShare.rightsWrite.clear();
        d.headIO.merge(assetShare);
        return true;
    }
}