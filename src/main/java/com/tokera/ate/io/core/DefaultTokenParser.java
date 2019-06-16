package com.tokera.ate.io.core;

import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ITokenParser;
import org.checkerframework.checker.nullness.qual.Nullable;

public class DefaultTokenParser implements ITokenParser {

    @Override
    public @Nullable IPartitionKey extractPartitionKey(TokenDto token) {
        return token.getPartitionKeyOrNull();
    }
}
