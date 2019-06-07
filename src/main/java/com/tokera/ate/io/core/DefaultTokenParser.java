package com.tokera.ate.io.core;

import com.tokera.ate.common.StringTools;
import com.tokera.ate.common.UUIDTools;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.TokenDto;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.api.ITokenParser;
import com.tokera.ate.units.DomainName;
import com.tokera.ate.units.EmailAddress;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.UUID;

public class DefaultTokenParser implements ITokenParser {

    @Override
    public @Nullable IPartitionKey extractPartitionKey(TokenDto token) {
        IPartitionKey key = token.getPartitionKeyOrNull();
        if (key != null) {
            return key;
        }

        @EmailAddress String email = token.getUsername();
        @DomainName String domain = StringTools.getDomain(email);
        UUID id = UUIDTools.generateUUID(domain);
        return AteDelegate.get().io.partitionKeyMapper().resolve(id);
    }
}
