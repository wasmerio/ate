package com.tokera.ate.constraints;

import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dao.enumerations.KeyUse;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import com.tokera.ate.dto.msg.MessageSecurityCastleDto;
import org.apache.commons.codec.binary.Base64;
import org.bouncycastle.crypto.InvalidCipherTextException;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.ConstraintValidator;
import javax.validation.ConstraintValidatorContext;
import java.io.IOException;
import java.util.Arrays;

/**
 * Validator that holds a bunch of rules that determine if a private key is valid
 */
public class CastleValidator implements ConstraintValidator<CastleConstraint, @Nullable MessageSecurityCastleDto> {

    @Override
    public void initialize(CastleConstraint constraintAnnotation) {
    }

    @Override
    public boolean isValid(@Nullable MessageSecurityCastleDto castle, ConstraintValidatorContext constraintValidatorContext) {
        if (castle == null) return true;
        boolean ret = true;

        if (castle.getGates().size() <= 0) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The castle has no gates.").addConstraintViolation();
            ret = false;
        }
        if (castle.getLookup().size() <= 0) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The castle has no lookups.").addConstraintViolation();
            ret = false;
        }

        return ret;
    }
}