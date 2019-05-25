package com.tokera.examples.dto;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dao.AssetShare;

public class ShareToken {
    public final PUUID share;
    public final MessagePrivateKeyDto writeRight;
    public final MessagePrivateKeyDto readRight;

    public ShareToken(AssetShare assetShare) {
        this.share = assetShare.addressableId();
        AteDelegate d = AteDelegate.get();
        this.writeRight = d.authorization.getOrCreateImplicitRightToWrite(assetShare);
        this.readRight = d.authorization.getOrCreateImplicitRightToRead(assetShare);
    }
}