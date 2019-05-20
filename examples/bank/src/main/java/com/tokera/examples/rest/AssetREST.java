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

@ApplicationScoped
@Path("/asset")
@PermitReadEntity(name="accountId", clazz= Account.class)
public class AssetREST {
    protected AteDelegate d = AteDelegate.get();

    @POST
    @Produces({"text/yaml", MediaType.APPLICATION_JSON})
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public TransactionToken create(CreateAssetRequest request) {
        Asset asset = new Asset(request.type, request.value);

        Share share = new Share(asset, request.value);
        share.trustInheritRead = true;
        share.trustInheritWrite = false;
        d.authorization.authorizeEntity(share, share);

        d.headIO.mergeLater(asset);
        d.headIO.mergeLater(share);
        return new TransactionToken(Collections.singletonList(new ShareToken(share)));
    }

    @POST
    @Produces(MediaType.TEXT_PLAIN)
    @Consumes({"text/yaml", MediaType.APPLICATION_JSON})
    public boolean redeem(RedeemAssetRequest request) {
        Share share = d.headIO.get(request.shareToken.share, Share.class);
        d.daoHelper.hasImplicitAuthority(share, request.validateType);
        share.rightsWrite.clear();
        d.headIO.merge(share);
        return true;
    }
}