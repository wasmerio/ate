package com.tokera.ate.common;

import com.tokera.ate.exceptions.ErrorDetail;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import javax.validation.ConstraintViolation;
import javax.validation.Validation;
import javax.validation.Validator;
import javax.validation.ValidatorFactory;

/**
 * Provides validation utilities to validate Beans that conform to the Java
 * annotation validation framework.
 *
 * @author jonhanlee
 */
public class ValidationUtil {

    /**
     * Validates a bean that uses the Java annotation validation framework
     *
     * @param bean bean to be validated
     * @return error message
     */
    public static List<ErrorDetail> validateBean(Object bean) {
        ValidatorFactory factory = Validation.buildDefaultValidatorFactory();
        Validator validator = factory.getValidator();
        Set<ConstraintViolation<Object>> constraintViolations = validator.validate(bean);
        if (constraintViolations.isEmpty()) {
            return new ArrayList();
        }
        return getValidationErrorMessage(constraintViolations);
    }

    /**
     * Formats collection of Constraint violations into a JSON string
     *
     * @param constraintViolations set of constraint violations
     * @return validation error message in one string
     */
    public static List<ErrorDetail> getValidationErrorMessage(Set<ConstraintViolation<Object>> constraintViolations) {
        List<ErrorDetail> errorDetails = new ArrayList<>();
        for (ConstraintViolation<?> cv : constraintViolations) {
            errorDetails.add(new ErrorDetail(cv.getPropertyPath().toString(), cv.getMessage()));
        }
        return errorDetails;
    }
}
