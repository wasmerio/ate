package com.tokera.ate.common;

import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.units.DomainName;
import com.tokera.ate.units.EmailAddress;
import com.tokera.ate.units.Filepath;
import com.tokera.ate.units.LogText;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.ClientErrorException;
import javax.ws.rs.core.Response;
import java.io.*;
import java.util.List;
import java.util.Scanner;
import java.util.zip.DeflaterOutputStream;
import java.util.zip.InflaterInputStream;

/**
 * Helper class that compresses strings, prefixes, makes them pretty and other cool functions
 */
public class StringTools
{    
    public static String prettyString(String text)
    {
        StringBuilder sb = new StringBuilder();
        while (text.length() > 64) {
            sb.append(text.substring(0, 64)).append("\n");
            text = text.substring(64);
        }
        if (text.length() > 0)
            sb.append(text);
        return sb.toString();
    }

    public static byte[] compress(String text) {
        try (ByteArrayOutputStream baos = new ByteArrayOutputStream()) {
            try (OutputStream out = new DeflaterOutputStream(baos)) {
                out.write(text.getBytes("UTF-8"));
            }
            return baos.toByteArray();
        } catch (IOException ex) {
            throw new RuntimeException("Exception occured while compressing text", ex);
        }
    }

    public static String decompress(byte[] bytes) throws IOException {
        try (ByteArrayOutputStream baos = new ByteArrayOutputStream();
                ByteArrayInputStream bais = new ByteArrayInputStream(bytes);
                InputStream in = new InflaterInputStream(bais)) {
            byte[] buffer = new byte[8192];
            int len;
            while ((len = in.read(buffer)) > 0) {
                baos.write(buffer, 0, len);
            }
            return new String(baos.toByteArray(), "UTF-8");
        }
    }
    
    public static @LogText String toString(List<RuntimeException> errors)
    {
        // If an exception occured then write them to the error buffer before throwing an aggregate
        if (errors.size() > 0)
        {
            StringBuilder sb = new StringBuilder();
            for (RuntimeException ex : errors) {
                if (sb.length() > 0) sb.append("\n");
                sb.append(ex.getMessage());
            }
            return sb.toString();
        }

        return "";
    }

    public static @LogText String toString(Throwable ex)
    {
        StringWriter stack = new StringWriter();
        ex.printStackTrace(new PrintWriter(stack));
        return ex.getMessage() != null ? ex.getMessage() + "\n" + stack.toString() : stack.toString();
    }

    public static @DomainName String getDomain(@EmailAddress String email)
    {
        String[] comps = email.split("@");
        if (comps.length != 2) {
            throw new RuntimeException("Email address is not well formed [" + email + "].");
        }

        String username = comps[0];
        String domain = comps[1];
        return domain;
    }

    public static @Nullable @DomainName String getDomainOrNull(@Nullable @EmailAddress String _email)
    {
        String email = _email;
        if (email == null) return null;

        String[] comps = email.split("@");
        if (comps.length != 2) {
            return null;
        }

        String username = comps[0];
        String domain = comps[1];
        return domain;
    }

    public static @DomainName String getPrivateDomain(@Nullable @EmailAddress String _email)
    {
        @EmailAddress String email = _email;
        if (email == null) throw new RuntimeException("Email address is not value.");
        String[] comps = email.split("@");
        if (comps.length != 2) {
            throw new RuntimeException("Email address is not well formed.");
        }

        String username = comps[0];
        return username + ".at." + StringTools.getDomain(email);
    }

    @SuppressWarnings( "deprecation" )
    public static String unescapeLines(String body)
    {
        StringBuilder sb = new StringBuilder();
        Scanner scanner = new Scanner(body);
        for (;scanner.hasNextLine();) {
            String line = scanner.nextLine();
            if (line.startsWith("\"") && line.endsWith("\"")) {
                sb.append(org.apache.commons.lang3.StringEscapeUtils.unescapeJava(line)).append("\n");
            } else {
                sb.append(line).append("\n");
            }
        }
        return sb.toString();
    }

    public static String prefixLines(String body, String prefix)
    {
        StringBuilder sb = new StringBuilder();
        Scanner scanner = new Scanner(body);
        for (;scanner.hasNextLine();) {
            String line = scanner.nextLine();
            sb.append(prefix).append(line).append("\n");
        }
        return sb.toString();
    }

    public static boolean endsWithNewline(String line) {
        if (line.length() <= 0) return false;
        char lastChar = line.charAt(line.length() - 1);
        return lastChar == '\r' ||lastChar == '\n';
    }

    public static boolean endsWithNewline(StringBuilder line) {
        if (line.length() <= 0) return false;
        char lastChar = line.charAt(line.length() - 1);
        return lastChar == '\r' ||lastChar == '\n';
    }

    public static void sanitizePath(@Nullable @Filepath String path) {
        if (path == null) return;
        if (path.contains("..")) {
            throw new RuntimeException("This path [" + path + "] is a security risk.");
        }
    }

    public static @Nullable String makeOneLineOrNull(@Nullable String value) {
        if (value == null) return null;
        return value.replace("\r", "").replace("\n", "").replace("\0", "");
    }

    public static String makeOneLine(String value) {
        return value.replace("\r", "").replace("\n", "").replace("\0", "");
    }

    public static @Nullable String specialParse(@Nullable String value)
    {
        if (value != null) {
            String valueClean = StringTools.makeOneLine(value);
            if (valueClean.equals("[null]") ||
                    valueClean.equals("null") ||
                    valueClean.length() <= 0)
            {
                value = null;
            }
        }

        return value;
    }

    public static String toList(List<String> lines) {
        return toList(lines, null);
    }

    public static String toList(List<String> lines, @Nullable String prefix) {
        StringBuilder sb = new StringBuilder();
        for (String line : lines) {
            if (sb.length() > 0) sb.append("\n");
            if (prefix != null) sb.append(prefix);
            sb.append(line);
        }
        return sb.toString();
    }

    public static String toMsg(ClientErrorException e)
    {
        Response resp = e.getResponse();
        Object msg;
        msg = resp.getEntity();
        if (msg instanceof InputStream) {
            msg = null;

            InputStream stream = (InputStream)msg;
            if (stream != null) {
                try {
                    Scanner scanner = new Scanner(stream);
                    scanner.useDelimiter("\\Z");
                    String data = "";
                    if (scanner.hasNext()) data = scanner.next();
                    if (data.length() > 0) msg = data;
                } catch (Throwable e1) {
                    AteDelegate.get().genericLogger.warn(e1.getMessage(), e1);
                }
            }
        }
        if (msg == null) msg = e.getMessage();
        if (msg == null) msg = e.getClass().getSimpleName();
        resp.close();
        return msg.toString();
    }
}
