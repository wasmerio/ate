package com.tokera.ate.constraints;

import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
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
        boolean ret = true;
        if (key.getPublicParts() == null) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key has no public parts.").addConstraintViolation();
            ret = false;
        }
        if (key.getPublicParts().size() <= 0) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key public parts are empty.").addConstraintViolation();
            ret = false;
        }
        if (key.getPublicKeyHash() == null) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key has no public key hash.").addConstraintViolation();
            ret = false;
        }
        for (MessageKeyPartDto part : key.getPublicParts()) {
            if (part.getType() == KeyType.unknown) {
                if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
                constraintValidatorContext.buildConstraintViolationWithTemplate("The key has public parts that use an unknown crypto algorithm.").addConstraintViolation();
                ret = false;
            }
        }
        return ret;
    }
}