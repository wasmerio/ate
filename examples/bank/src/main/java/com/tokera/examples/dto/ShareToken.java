package com.tokera.examples.dto;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dao.Share;

public class ShareToken {
    public final PUUID share;
    public final MessagePrivateKeyDto writeRight;
    public final MessagePrivateKeyDto readRight;

    public ShareToken(Share share) {
        this.share = share.addressableId();
        AteDelegate d = AteDelegate.get();
        this.writeRight = d.authorization.getOrCreateImplicitRightToWrite(share);
        this.readRight = d.authorization.getOrCreateImplicitRightToRead(share);
    }
}