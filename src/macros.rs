#[macro_export]
macro_rules! pln {
    () => {
        console::Term::stdout().write_line("").unwrap();
    };
    ($($arg:tt)*) => {{
        console::Term::stdout().write_line(&format!($($arg)*)).unwrap()
    }};
}

#[macro_export]
macro_rules! p {
    ($($arg:tt)*) => {{
        console::Term::stdout().write_str(&format!($($arg)*)).unwrap()
    }};
}

#[macro_export]
macro_rules! clear_p {
    ($($arg:tt)*) => {{
        console::Term::stdout().clear_line().unwrap();
        console::Term::stdout().write_str(&format!($($arg)*)).unwrap()
    }};
}

#[macro_export]
macro_rules! clear_pln {
    () => {
        console::Term::stdout().clear_line().unwrap();
        console::Term::stdout().write_line("").unwrap();
    };
    ($($arg:tt)*) => {{
        console::Term::stdout().clear_line().unwrap();
        console::Term::stdout().write_line(&format!($($arg)*)).unwrap()
    }};
}
