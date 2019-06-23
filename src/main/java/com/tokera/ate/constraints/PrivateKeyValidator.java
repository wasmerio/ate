package com.tokera.ate.constraints;

import com.tokera.ate.dao.enumerations.KeyType;
import com.tokera.ate.dao.enumerations.KeyUse;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.msg.MessageKeyPartDto;
import com.tokera.ate.dto.msg.MessagePrivateKeyDto;
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

        try {
            AteDelegate d = AteDelegate.get();
            if (d.bootstrapConfig.isExtraValidation()) {
                if (key.getPrivateParts().stream().anyMatch(p -> p.getType().getUse() == KeyUse.encrypt)) {
                    byte[] plain = Base64.decodeBase64(d.encryptor.generateSecret64());
                    byte[] enc = d.encryptor.encrypt(key, plain);
                    byte[] plain2 = d.encryptor.decrypt(key, enc);
                    if (Arrays.equals(plain, plain2) == false) {
                        if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
                        constraintValidatorContext.buildConstraintViolationWithTemplate("The key did not pass the encrypt/decrypt test.").addConstraintViolation();
                        ret = false;
                    }
                }
                if (key.getPrivateParts().stream().anyMatch(p -> p.getType().getUse() == KeyUse.sign)) {
                    byte[] plain = Base64.decodeBase64(d.encryptor.generateSecret64());
                    byte[] sig = d.encryptor.sign(key, plain);
                    if (d.encryptor.verify(key, plain, sig) == false) {
                        if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
                        constraintValidatorContext.buildConstraintViolationWithTemplate("The key did not pass the sign/verify test.").addConstraintViolation();
                        ret = false;
                    }
                }
            }
        } catch (IOException | InvalidCipherTextException e) {
            if (ret == true) constraintValidatorContext.disableDefaultConstraintViolation();
            constraintValidatorContext.buildConstraintViolationWithTemplate("The key did not pass the testing phase - " + e.getMessage()).addConstraintViolation();
            ret = false;
        }

        return ret;
    }
}