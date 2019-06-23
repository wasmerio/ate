package com.tokera.ate.constraints;

import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
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

        if (key.getPrivateParts() == null) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key has no private parts.").addConstraintViolation();
            ret = false;
        }
        if (key.getPrivateParts().size() <= 0) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key private parts are empty.").addConstraintViolation();
            ret = false;
        }
        if (key.getPrivateKeyHash() == null) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key has no private key hash.").addConstraintViolation();
            ret = false;
        }
        for (MessageKeyPartDto part : key.getPrivateParts()) {
            if (part.getType() == KeyType.unknown) {
                if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
                constraintValidatorContext.buildConstraintViolationWithTemplate("The key has private parts that use an unknown crypto algorithm.").addConstraintViolation();
                ret = false;
            }
        }
        return ret;
    }
}