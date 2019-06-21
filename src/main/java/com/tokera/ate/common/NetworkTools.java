package com.tokera.ate.common;

import org.checkerframework.checker.nullness.qual.Nullable;

import java.net.*;
import java.util.Collections;
import java.util.HashSet;
import java.util.Set;

public class NetworkTools {

    private static Set<String> externalNetworkAddresses = new HashSet<>();

    public static Set<String> getMyNetworkAddresses() {
        HashSet<String> myAddresses = new HashSet<>();
        try {
            for (NetworkInterface net : Collections.list(NetworkInterface.getNetworkInterfaces())) {
                for (InetAddress addr : Collections.list(net.getInetAddresses())) {
                    String addrStr = addr.toString().trim().toLowerCase();
                    if (addrStr.startsWith("/")) addrStr = addrStr.substring(1);
                    if (addrStr.contains("%")) {
                        addrStr = addrStr.split("%")[0];
                    }
                    if (addr instanceof Inet6Address) {
                        addrStr = "[" + addrStr + "]";
                    }
                    myAddresses.add(addrStr);
                }
            }
        } catch (SocketException e) {
            try {
                myAddresses.add(InetAddress.getLocalHost().toString());
            } catch (UnknownHostException e1) {
                throw new RuntimeException("Failed to determine the IP address of this machine.", e1);
            }
        }

        myAddresses.addAll(externalNetworkAddresses);
        return myAddresses;
    }

    public static @Nullable String extractAddressFromBootstrap(@Nullable String bootstrap) {
        if (bootstrap == null) return null;
        if (bootstrap.contains(":") == false) return bootstrap;
        String[] comps = bootstrap.split(":");
        if (comps.length < 2) return bootstrap;
        return comps[0];
    }

    public static String extractAddressFromBootstrapOrThrow(String bootstrap) {
        String ret = extractAddressFromBootstrap(bootstrap);
        if (ret == null) {
            throw new RuntimeException("Failed to determine the address from the bootstrap [" + bootstrap + "] - ensure a port number of post-fixed on the end separated with a colon.");
        }
        return ret;
    }

    public static @Nullable Integer extractPortFromBootstrap(@Nullable String bootstrap) {
        if (bootstrap == null) return null;
        String[] comps = bootstrap.split(":");
        if (comps.length < 2) return null;
        return Integer.parseInt(comps[1]);
    }

    public static Integer extractPortFromBootstrapOrThrow(String bootstrap) {
        Integer ret = extractPortFromBootstrap(bootstrap);
        if (ret == null) {
            throw new RuntimeException("Failed to determine the port from the address [" + bootstrap + "] - ensure a port number of post-fixed on the end separated with a colon.");
        }
        return ret;
    }

    public static void addExternalNetworkAddress(String addr) {
        externalNetworkAddresses.add(addr);
    }
}
