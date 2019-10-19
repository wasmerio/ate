package com.tokera.ate.common;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.exceptions.ErrorDetail;
import com.tokera.ate.scopes.Startup;

import java.io.PrintWriter;
import java.io.StringWriter;
import java.util.*;
import javax.annotation.Nullable;
import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.validation.*;
import javax.ws.rs.WebApplicationException;
import javax.ws.rs.core.Response;

/**
 * Provides validation utilities to validate Beans that conform to the Java
 * annotation validation framework.
 */
@Startup
@ApplicationScoped
public class ValidationUtil {
    private AteDelegate d = AteDelegate.get();
    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private Validator validator;

    @PostConstruct
    public void init() {
        ValidatorFactory factory = Validation.buildDefaultValidatorFactory();
        validator = factory.getValidator();
    }

    /**
     * Validates a bean that uses the Java annotation validation framework
     *
     * @param obj bean to be validated
     * @return error message
     */
    public List<ErrorDetail> validate(Object obj) {
        Set<ConstraintViolation<Object>> constraintViolations = validator.validate(obj);
        if (constraintViolations.isEmpty()) {
            return Collections.emptyList();
        }
        return getValidationErrorMessage(constraintViolations);
    }

    /**
     * Throws an exception if the bean does not validate properly
     */
    public boolean validateOrLog(Object obj, @Nullable LoggerHook LOG) {
        List<ErrorDetail> errors = validate(obj);
        if (errors.size() > 0) {
            String msg = convertValidationErrorDetails(obj, errors);
            if (LOG != null) LOG.info(msg);
            else d.genericLogger.info(msg);
            return false;
        }
        return true;
    }

    /**
     * Throws an exception if the bean does not validate properly
     */
    public void validateOrThrow(Object obj) {
        List<ErrorDetail> errors = validate(obj);
        if (errors.size() > 0) {
            String msg = convertValidationErrorDetails(obj, errors);
            throw new WebApplicationException(msg, Response.Status.BAD_REQUEST);
        }
    }

    /**
     * Converts a list of error details into a message
     **/
    public String convertValidationErrorDetails(Object obj, List<ErrorDetail> errors) {
        StringBuilder sb = new StringBuilder();
        sb.append(obj.getClass().getSimpleName() + " failed validation:\n");
        for (ErrorDetail detail : errors) {
            sb.append("- " + detail.getMessage());
        }

        if (d.bootstrapConfig.isLoggingValidationVerbose()) {
            String objTxt;
            try {
                objTxt = d.yaml.serializeObj(obj);
            } catch (Throwable ex) {
                objTxt = "[serialization failed]";
            }

            try {
                throw new ValidationException(sb.toString());
            } catch (Throwable ex) {
                StringWriter sw = new StringWriter();
                ex.printStackTrace(new PrintWriter(sw));
                return ex.getMessage() + "\n" + objTxt + "\n" + sw.toString();
            }
        }

        return sb.toString();
    }

    /**
     * Formats collection of Constraint violations into a JSON string
     *
     * @param constraintViolations set of constraint violations
     * @return validation error message in one string
     */
    public List<ErrorDetail> getValidationErrorMessage(Set<ConstraintViolation<Object>> constraintViolations) {
        List<ErrorDetail> errorDetails = new ArrayList<>();
        for (ConstraintViolation<?> cv : constraintViolations) {
            errorDetails.add(new ErrorDetail(cv.getPropertyPath().toString(), cv.getMessage()));
        }
        return errorDetails;
    }
}
