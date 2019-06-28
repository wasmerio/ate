package com.tokera.ate.constraints;

import javax.validation.Constraint;
import javax.validation.Payload;
import java.lang.annotation.Documented;
import java.lang.annotation.Retention;
import java.lang.annotation.Target;

import static java.lang.annotation.ElementType.ANNOTATION_TYPE;
import static java.lang.annotation.ElementType.TYPE;
import static java.lang.annotation.RetentionPolicy.RUNTIME;

/**
 * Validation constraint that ensures the castle objects are valid
 */
@Target({ TYPE, ANNOTATION_TYPE })
@Retention(RUNTIME)
@Constraint(validatedBy = { CastleValidator.class })
@Documented
public @interface CastleConstraint {
    String message() default "Castle is not valid (check that it has all required fields)";

    Class<?>[] groups() default { };

    Class<? extends Payload>[] payload() default { };
}