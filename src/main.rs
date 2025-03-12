#![feature(if_let_guard)]

mod config;
mod error;

use config::Config;
use std::sync::LazyLock;

static CONFIG: LazyLock<Config> = LazyLock::new(|| 
	Config::deser("rustfmt.toml").unwrap_or_default());

#[derive(Debug, Clone, Copy, Default)]
struct Args {
	dry: bool,
}

fn main() {
	#[cfg(not(debug_assertions))]
	std::panic::set_hook(Box::new(|e| {
		match e.payload() {
			p if let Some(s) = p.downcast_ref::<&str>()   => warn!("{s}"),
			p if let Some(s) = p.downcast_ref::<String>() => warn!("{s}"),
			_ => warn!("SHIT WENT DOWN"),
		}
	}));

	LazyLock::force(&CONFIG);

	let mut args = Args::default();

	let files = std::env::args().skip(1)
		.filter(|p| match p.strip_prefix('-') {
			Some("d" | "dry") => { args.dry = true; false },
			_ => true,
		})
		.filter_map(|p| std::fs::read_to_string(&p)
			.inspect_err(|e| warn!("{e}"))
			.map(|s| (p, s))
			.ok())
		.filter_map(|(p, s)| syn::parse_file(&s)
			.inspect_err(|e| warn!("{e}"))
			.map(|f| (p, f))
			.ok())
		.filter_map(|(p, f)| std::panic::catch_unwind(|| process_file(f))
			.map(|s| (p, s))
			.ok())
		.collect::<Vec<_>>();

	match args.dry {
		true => files.iter()
			.for_each(|(p, s)| println!("=== {p} ===\n{s}")),
		false => files.iter()
			.for_each(|(p, s)| std::fs::write(p, s)
			.inspect_err(|e| warn!("{e}"))
			.unwrap()),
	}
}

use std::cell::Cell;

thread_local! {
	static INDENT: Cell<usize> = const { Cell::new(0) };
}

fn process_file(file: syn::File) -> String {
	file.items.into_iter().fold(String::new(), |mut s, i| {
		match i {
			syn::Item::Fn(f) => rewrite_fn(&mut s, &f),
			_ => unreachable!(),
		}
		s
	})
}

fn rewrite_fn(s: &mut String, f: &syn::ItemFn) {
	if f.sig.constness.is_some() { s.push_str("const ") }
	s.push_str("fn ");
	s.push_str(&f.sig.ident.to_string());
	s.push('(');

	for (i, p) in f.sig.inputs.iter().enumerate() {
		if i > 0 { s.push_str(", ") }
		match p {
			syn::FnArg::Receiver(r) => {
				if r.reference.is_some()  { s.push('&') }
				if r.mutability.is_some() { s.push_str("mut ") }
				s.push_str("self ");
				if r.colon_token.is_some() {
					s.push_str(": ");
					rewite_type(s, &r.ty);
				}
			},
			syn::FnArg::Typed(p) => {
				if let syn::Pat::Ident(p) = &*p.pat {
					s.push_str(&p.ident.to_string());
				}
			}
		}
	}

	s.push_str(") ");

	if let syn::ReturnType::Type(_, t) = &f.sig.output {
		s.push_str("-> ");
		rewite_type(s, t);
	}

	rewrite_block(s, &f.block);
}

fn rewite_type(s: &mut String, t: &syn::Type) {
	use syn::Type;
	match t {
		Type::Ptr(p) => {
			s.push('*');
			if p.const_token.is_some() { s.push_str("const ") }
			if p.mutability.is_some() { s.push_str("mut ") }
			rewite_type(s, &p.elem);
		},
		Type::Path(p) => rewrite_path(s, &p.path),
		_ => todo!(),
	}
}

fn rewrite_path(s: &mut String, p: &syn::Path) {
	for (i, seg) in p.segments.iter().enumerate() {
		if i > 0 { s.push_str("::") }
		s.push_str(&seg.ident.to_string());
	}
}

fn rewrite_block(s: &mut String, p: &syn::Block) {
	let process_stmt = |s: &mut String, stmt: &syn::Stmt|
		match stmt {
			syn::Stmt::Item(i)     => rewrite_item(s, i),
			syn::Stmt::Expr(e, se) => rewrite_expr(s, e, se.is_some()),
			_ => todo!(),
		};

	match p.stmts.len() {
		2.. => s.push_str("{\n"),
		1   => {
			s.push_str("{ ");
			process_stmt(s, &p.stmts[0]);
			s.push_str(" }");
			return;
		},
		0   => { 
			s.push_str("{ }");
			return;
		},
	}

	INDENT.set(INDENT.get() + 1);

	for stmt in &p.stmts {
		s.push_str(&CONFIG.indent().repeat(INDENT.get()));
		process_stmt(s, stmt);
		s.push('\n');
	}

	INDENT.set(INDENT.get() - 1);

	s.push('}');
}

fn rewrite_expr(s: &mut String, e: &syn::Expr, semi: bool) {
	use syn::Expr;
	match e {
		Expr::Block(b) => rewrite_block(s, &b.block),
		Expr::Call(c) => {
			rewrite_expr(s, &c.func, false);
			s.push('(');
			for (i, arg) in c.args.iter().enumerate() {
				if i > 0 { s.push_str(", ") }
				rewrite_expr(s, arg, false);
			}
			s.push(')');
		},
		Expr::Path(p) => rewrite_path(s, &p.path),
		_ => todo!(),
	}

	if semi { s.push(';') }
}

fn rewrite_item(s: &mut String, i: &syn::Item) {
	use syn::Item;
	match i {
		Item::Fn(f) => rewrite_fn(s, f),
		_ => todo!(),
	}
}
