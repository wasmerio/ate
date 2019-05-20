package com.tokera.examples.dto;

public class RedeemAssetRequest {
    public ShareToken shareToken;
    public String validateType;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public RedeemAssetRequest() {
    }

    public RedeemAssetRequest(ShareToken shareToken, String validateType) {
        this.shareToken = shareToken;
        this.validateType = validateType;
    }
}
