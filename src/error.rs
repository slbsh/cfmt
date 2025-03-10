#[macro_export]
macro_rules! err {
	($($ident:tt)*) => {{
		use colored::Colorize;
		eprintln!("{} {}", "ERR:".bold().red(), format!($($ident)*).red());
		std::process::exit(1)
	}};
}

#[macro_export]
macro_rules! warn {
	($($ident:tt)*) => {{
		use colored::Colorize;
		eprintln!("{} {}", "WARN:".bold().yellow(), format!($($ident)*).yellow());
	}};
}
