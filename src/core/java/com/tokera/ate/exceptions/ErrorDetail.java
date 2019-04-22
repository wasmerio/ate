package com.tokera.ate.exceptions;

/**
 *
 * @author jonhanlee
 */
public class ErrorDetail {

    private String source;
    private String message;

    public ErrorDetail(String source, String message) {
        this.source = source;
        this.message = message;
    }

    public String getSource() {
        return source;
    }

    public void setSource(String source) {
        this.source = source;
    }

    public String getMessage() {
        return message;
    }

    public void setMessage(String message) {
        this.message = message;
    }

    @Override
    public String toString() {
        return "ErrorDetail{" + "source=" + source + ", message=" + message + '}';
    }

}
