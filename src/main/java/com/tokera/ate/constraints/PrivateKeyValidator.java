package com.tokera.ate.constraints;

import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.validation.ConstraintValidator;
import javax.validation.ConstraintValidatorContext;

/**
 * Validator that holds a bunch of rules that determine if a private key is valid
 */
public class PrivateKeyValidator implements ConstraintValidator<PrivateKeyConstraint, @Nullable MessagePrivateKeyDto> {

    @Override
    public void initialize(PrivateKeyConstraint constraintAnnotation) {
    }

    @Override
    public boolean isValid(@Nullable MessagePrivateKeyDto key, ConstraintValidatorContext constraintValidatorContext) {
        if (key == null) return true;
        if (key.getPublicParts() == null) {
            constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key has no public parts.").addConstraintViolation();
            return false;
        }
        if (key.getPublicParts().size() <= 0) {
            constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key public parts are empty.").addConstraintViolation();
            return false;
        }
        if (key.getPublicKeyHash() == null) {
            constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key has no public key hash.").addConstraintViolation();
            return false;
        }
        if (key.getPrivateParts() == null) {
            constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key has no private parts.").addConstraintViolation();
            return false;
        }
        if (key.getPrivateParts().size() <= 0) {
            constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key private parts are empty.").addConstraintViolation();
            return false;
        }
        if (key.getPrivateKeyHash() == null) {
            constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key has no private key hash.").addConstraintViolation();
            return false;
        }
        return true;
    }
}