package com.tokera.examples.dto;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dao.AssetShare;

public class ShareToken {
    private PUUID share;
    private MessagePrivateKeyDto ownership;

    public ShareToken() {
    }

    public ShareToken(AssetShare assetShare, MessagePrivateKeyDto ownership) {
        this.share = assetShare.addressableId();
        AteDelegate d = AteDelegate.get();

        this.ownership = ownership;
        assetShare.encryptKey = null;
        assetShare.trustAllowWrite.clear();
        d.authorization.authorizeEntityWrite(this.ownership, assetShare);
    }

    public PUUID getShare() {
        return share;
    }

    public void setShare(PUUID share) {
        this.share = share;
    }

    public MessagePrivateKeyDto getOwnership() {
        return ownership;
    }

    public void setOwnership(MessagePrivateKeyDto ownership) {
        this.ownership = ownership;
    }
}