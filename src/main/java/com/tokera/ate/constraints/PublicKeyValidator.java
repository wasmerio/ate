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
        return true;
    }
}