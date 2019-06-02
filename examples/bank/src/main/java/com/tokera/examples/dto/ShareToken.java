package com.tokera.examples.dto;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dao.AssetShare;

public class ShareToken {
    private PUUID share;
    private MessagePrivateKeyDto writeRight;
    private MessagePrivateKeyDto readRight;

    public ShareToken() {
    }

    public ShareToken(AssetShare assetShare) {
        this.share = assetShare.addressableId();
        AteDelegate d = AteDelegate.get();
        this.writeRight = d.authorization.getOrCreateImplicitRightToWrite(assetShare);
        this.readRight = d.authorization.getOrCreateImplicitRightToRead(assetShare);
    }

    public PUUID getShare() {
        return share;
    }

    public void setShare(PUUID share) {
        this.share = share;
    }

    public MessagePrivateKeyDto getWriteRight() {
        return writeRight;
    }

    public void setWriteRight(MessagePrivateKeyDto writeRight) {
        this.writeRight = writeRight;
    }

    public MessagePrivateKeyDto getReadRight() {
        return readRight;
    }

    public void setReadRight(MessagePrivateKeyDto readRight) {
        this.readRight = readRight;
    }
}