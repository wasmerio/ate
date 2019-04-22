package com.tokera.ate.constraints;

import com.tokera.ate.dto.msg.MessagePublicKeyDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.ConstraintValidator;
import javax.validation.ConstraintValidatorContext;

/**
 * Validator that will check a bunch of rules to see if a public key is valid or not
 */
public class PublicKeyValidator implements ConstraintValidator<PublicKeyConstraint, @Nullable MessagePublicKeyDto> {

    @Override
    public void initialize(PublicKeyConstraint constraintAnnotation) {
    }

    @Override
    public boolean isValid(@Nullable MessagePublicKeyDto key, ConstraintValidatorContext constraintValidatorContext) {
        if (key == null) return true;
        if (key.getPublicKey() == null) return false;
        if (key.getPublicKeyBytes() == null) return false;
        if (key.getPublicKeyHash() == null) return false;
        return true;
    }
}