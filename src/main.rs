mod config;
mod error;

use config::Config;
use std::sync::LazyLock;

static CONFIG: LazyLock<Config> = LazyLock::new(|| 
	Config::deser("rustfmt.toml").unwrap_or_default());

fn main() {
	LazyLock::force(&CONFIG);

	std::env::args().skip(1)
		.filter_map(|p| std::fs::read_to_string(&p)
			.inspect_err(|e| warn!("{e}"))
			.map(|s| (p, s))
			.ok())
		.filter_map(|(p, s)| syn::parse_file(&s)
			.inspect_err(|e| warn!("{e}"))
			.map(|f| (p, f))
			.ok())
		.filter_map(|(p, f)| process_file(f)
			.inspect_err(|e| warn!("{e}"))
			.map(|s| (p, s))
			.ok())
		.for_each(|(p, s)| std::fs::write(p, s)
			.inspect_err(|e| warn!("{e}"))
			.unwrap());
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn str<T, R, F: FnOnce(&mut String, T) -> Result<R>>(f: F, t: T) -> Result<String> {
	let mut s = String::new();
	f(&mut s, t)?;
	Ok(s)
}

fn process_file(file: syn::File) -> Result<String> {
	file.items.into_iter().try_fold(String::new(), |mut s, i| {
		match i {
			syn::Item::Fn(f) => rewrite_fn(&mut s, f)?,
			_ => unreachable!(),
		}
		Ok(s)
	})
}

fn rewrite_fn(s: &mut String, f: syn::ItemFn) -> Result<()> {
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
					rewite_type(s, &r.ty)?;
				}

        // pub attrs: Vec<Attribute>,
        // pub reference: Option<(Token![&], Option<Lifetime>)>,
        // pub mutability: Option<Token![mut]>,
        // pub self_token: Token![self],
        // pub colon_token: Option<Token![:]>,
        // pub ty: Box<Type>,

			},
			syn::FnArg::Typed(p) => {
				if let syn::Pat::Ident(p) = &*p.pat {
					s.push_str(&p.ident.to_string());
				}
			}
		}
	}

	s.push(')');

	if let syn::ReturnType::Type(_, t) = &f.sig.output {
		s.push_str(" -> ");
		rewite_type(s, t)?;
	}

	rewrite_block(s, &f.block)?;

	Ok(())
}

fn rewite_type(s: &mut String, t: &syn::Type) -> Result<()> {
	use syn::Type;
	match t {
		Type::Ptr(p) => {
			s.push('*');
			if p.const_token.is_some() { s.push_str("const ") }
			if p.mutability.is_some() { s.push_str("mut ") }
			rewite_type(s, &p.elem)?;
		},
		Type::Path(p) => rewrite_path(s, &p.path)?,
		_ => todo!(),
	}
	Ok(())
}

fn rewrite_path(s: &mut String, p: &syn::Path) -> Result<()> {
	for (i, seg) in p.segments.iter().enumerate() {
		if i > 0 { s.push_str("::") }
		s.push_str(&seg.ident.to_string());
	}
	Ok(())
}

fn rewrite_block(s: &mut String, p: &syn::Block) -> Result<()> {
	s.push('{');
	if p.stmts.len() > 1 { s.push('\n') }

	for stmt in &p.stmts {
		match stmt {
			syn::Stmt::Item(i) => {
				match i {
					syn::Item::Expr(e) => rewrite_expr(s, e)?,
					_ => todo!(),
				}
			},
			syn::Stmt::Expr(e, _) => rewrite_expr(s, e)?,
			_ => todo!(),
		}
		if p.stmts.len() > 1 { s.push('\n') }
	}

	s.push('}');

	Ok(())
}

fn rewrite_expr(s: &mut String, e: &syn::Expr) -> Result<()> {
	use syn::Expr;
	match e {
		Expr::Block(b) => rewrite_block(s, &b.block)?,
		Expr::Call(c) => {
			rewrite_expr(s, &c.func)?;
			s.push('(');
			for (i, arg) in c.args.iter().enumerate() {
				if i > 0 { s.push_str(", ") }
				rewrite_expr(s, arg)?;
			}
			s.push(')');
		},
		Expr::Path(p) => rewrite_path(s, &p.path)?,
		_ => todo!(),
	}
	Ok(())
}
