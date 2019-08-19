package com.tokera.examples.dto;

import com.tokera.ate.dao.PUUID;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.PrivateKeyWithSeedDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.examples.dao.Coin;
import com.tokera.examples.dao.CoinShare;

public class ShareToken {
    private PUUID coin;
    private PUUID share;
    private PrivateKeyWithSeedDto ownership;

    public ShareToken() {
    }

    public ShareToken(Coin coin, CoinShare coinShare, PrivateKeyWithSeedDto ownership) {
        this.coin = coin.addressableId();
        this.share = coinShare.addressableId();
        AteDelegate d = AteDelegate.get();

        this.ownership = ownership;
        coinShare.trustAllowWrite.clear();
        d.authorization.authorizeEntityWrite(this.ownership, coinShare);
    }

    public PUUID getShare() {
        return share;
    }

    public void setShare(PUUID share) {
        this.share = share;
    }

    public PrivateKeyWithSeedDto getOwnership() {
        return ownership;
    }

    public void setOwnership(PrivateKeyWithSeedDto ownership) {
        this.ownership = ownership;
    }

    public PUUID getCoin() {
        return coin;
    }

    public void setCoin(PUUID coin) {
        this.coin = coin;
    }
}