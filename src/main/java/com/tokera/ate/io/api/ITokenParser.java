package com.tokera.ate.io.api;

import com.tokera.ate.dto.TokenDto;
import org.checkerframework.checker.nullness.qual.Nullable;

/**
 * Token parser is used to extract critical information from the tokens
 */
public interface ITokenParser {

    /**
     * @return The partition key extracted from the Token
     */
    @Nullable IPartitionKey extractPartitionKey(TokenDto token);
}
