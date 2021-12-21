pub type Pid = u32;

pub const MAX_MPSC: usize = std::usize::MAX >> 3;

pub fn is_cleared_line(text: &str) -> bool {
    // returns true if the displayed line is all blank on the screen
    text.ends_with("\r\x1b[0K") || text.ends_with("\x1b[0K\r") || text.ends_with("\n")
}

pub fn is_mobile(user_agent: &str) -> bool {
    user_agent.contains("Android")
        || user_agent.contains("BlackBerry")
        || user_agent.contains("iPhone")
        || user_agent.contains("iPad")
        || user_agent.contains("iPod")
        || user_agent.contains("Open Mini")
        || user_agent.contains("IEMobile")
        || user_agent.contains("WPDesktop")
}
