use super::*;

pub(super) fn eval_arg(env: &Environment, last_return: u32, arg: &str) -> String {
    if arg.as_bytes()[0] == b'$' {
        let key: &str = &arg[1..];
        match key {
            "?" => format!("{}", last_return),
            _ => match env.get(key) {
                Some(v) => v.clone(),
                None => String::new(),
            },
        }
    } else {
        arg.to_string()
    }
}
