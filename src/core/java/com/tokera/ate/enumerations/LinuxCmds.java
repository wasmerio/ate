package com.tokera.ate.enumerations;

import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.units.LinuxCmd;
import com.tokera.ate.units.LinuxError;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.function.Consumer;

public class LinuxCmds {
    public static final @LinuxCmd String Void = "";
    public static final @LinuxCmd String Yes = "1";
    public static final @LinuxCmd String No = "0";

    public static @LinuxCmd String clean(@LinuxCmd String cmd) {
        cmd = cmd.replace("\r", "").replace("\n", "");
        return cmd;
    }

    public static @LinuxCmd String fromBool(boolean val) {
        return val == true ? LinuxCmds.Yes : LinuxCmds.No;
    }

    public static <T extends BaseDao> @LinuxError String updateBool(@LinuxCmd String cmd, Consumer<Boolean> updateFn)
    {
        if (LinuxCmds.isYes(cmd)) {
            updateFn.accept(true);
            return LinuxErrors.OK;
        } else if (LinuxCmds.isNo(cmd)) {
            updateFn.accept(false);
            return LinuxErrors.OK;
        } else {
            return LinuxErrors.EINVAL;
        }
    }

    public static boolean isYes(@Nullable @LinuxCmd String _cmd) {
        @LinuxCmd String cmd = _cmd;
        if (cmd == null) return false;
        cmd = LinuxCmds.clean(cmd);
        return LinuxCmds.Yes.equalsIgnoreCase(cmd) ||
                "1".equalsIgnoreCase(cmd) ||
                "true".equalsIgnoreCase(cmd) ||
                "yes".equalsIgnoreCase(cmd) ||
                "ok".equalsIgnoreCase(cmd) ||
                "execute".equalsIgnoreCase(cmd) ||
                "run".equalsIgnoreCase(cmd);
    }
    public static boolean isNo(@Nullable @LinuxCmd String _cmd) {
        @LinuxCmd String cmd = _cmd;
        if (cmd == null) return false;
        cmd = LinuxCmds.clean(cmd);
        return LinuxCmds.No.equalsIgnoreCase(cmd) ||
                "0".equalsIgnoreCase(cmd) ||
                "false".equalsIgnoreCase(cmd) ||
                "no".equalsIgnoreCase(cmd) ||
                "cancel".equalsIgnoreCase(cmd) ||
                "nop".equalsIgnoreCase(cmd) ||
                "norun".equalsIgnoreCase(cmd);
    }
}
