use quote::{quote, quote_spanned};
use syn::{visit_mut::VisitMut, spanned::Spanned};
use std::collections::HashMap;
use proc_macro2::{TokenStream, TokenTree};
use itertools::Itertools;

#[derive(Default)]
struct Clown {
	honks: HashMap<syn::Ident, syn::Expr>,
	slips: HashMap<syn::Ident, syn::Expr>,
}

impl Clown {
	fn raw_tt_visit(&mut self, tt: TokenStream, acc: &mut TokenStream) {
		let mut tt_iter = tt.into_iter().multipeek();
		while let Some(tree) = tt_iter.next() {
			use std::iter::once;

			match tree {
				TokenTree::Group(group) => {
					let mut acc_rec = TokenStream::new();
					self.raw_tt_visit(group.stream(), &mut acc_rec);
					acc.extend(once(TokenTree::Group(proc_macro2::Group::new(group.delimiter(), acc_rec))));
				},
				TokenTree::Literal(_) | TokenTree::Punct(_) => acc.extend(once(tree)),
				TokenTree::Ident(ref ident) => {
					if ident == "honk" {
						let Some(TokenTree::Punct(punct)) = tt_iter.peek() else { acc.extend(once(tree)); continue; };
						if punct.as_char() != '!' { acc.extend(once(tree)); continue; }
						let Some(TokenTree::Group(group)) = tt_iter.peek() else { acc.extend(once(tree)); continue; };
						let expr = syn::parse2::<syn::Expr>(group.stream()).expect("honk! macro argument must be an expression");
						let ident = syn::Ident::new(&format!("__honk_{}", self.honks.len()), expr.span());
						acc.extend(once(TokenTree::Ident(ident.clone())));
						self.honks.insert(ident, expr);

						// we got `honk` as next, now skip `!` and group with args
						tt_iter.next(); tt_iter.next();
					} else if ident == "slip" {
						let Some(TokenTree::Punct(punct)) = tt_iter.peek() else { acc.extend(once(tree)); continue; };
						if punct.as_char() != '!' { acc.extend(once(tree)); continue; }
						let Some(TokenTree::Group(group)) = tt_iter.peek() else { acc.extend(once(tree)); continue; };
						let expr = syn::parse2::<syn::Expr>(group.stream()).expect("slip! macro argument must be an expression");
						let ident = syn::Ident::new(&format!("__slip_{}", self.slips.len()), expr.span());
						acc.extend(once(TokenTree::Ident(ident.clone())));
						self.slips.insert(ident, expr);

						// we got `slip` as next, now skip `!` and group with args
						tt_iter.next(); tt_iter.next();
					} else {
						acc.extend(once(tree));
					}
				},
			}
		}
	}
}

impl syn::visit_mut::VisitMut for Clown {
	fn visit_macro_mut(&mut self, this_macro: &mut syn::Macro) {
		let this_macro_tokens = std::mem::take(&mut this_macro.tokens);
		self.raw_tt_visit(this_macro_tokens, &mut this_macro.tokens);
	}

	fn visit_expr_mut(&mut self, this_expr: &mut syn::Expr) {
		match this_expr {
			syn::Expr::Macro(syn::ExprMacro { mac, .. }) => {
				let Some(mac_ident) = mac.path.get_ident() else { return syn::visit_mut::visit_expr_mut(self, this_expr); };
				if mac_ident == "honk" {
					let expr = syn::parse2::<syn::Expr>(mac.tokens.clone()).expect("honk! macro argument must be an expression");
					let ident = syn::Ident::new(&format!("__honk_{}", self.honks.len()), this_expr.span());
					*this_expr = syn::parse_quote!(#ident);
					self.honks.insert(ident, expr);
				} else if mac_ident == "slip" {
					let expr = syn::parse2::<syn::Expr>(mac.tokens.clone()).expect("slip! macro argument must be an expression");
					let ident = syn::Ident::new(&format!("__slip_{}", self.slips.len()), this_expr.span());
					*this_expr = syn::parse_quote!(#ident);
					self.slips.insert(ident, expr);
				}
			},
			// do not recurse into closures in case there's a nested clown
			syn::Expr::Closure(_) => {},
			_ => syn::visit_mut::visit_expr_mut(self, this_expr),
		}
	}
}

/// expands `#[clown] || do_call(honk!(foo.bar), slip!(baz.bop))` to `{ let __honk_0 = (foo.bar).clone(); let __slip_0 = baz.bop; move || do_call(__honk_0, __slip_0) }` as some approximation of "capture-by-clone" closure
#[proc_macro_attribute]
pub fn clown(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let mut item = syn::parse_macro_input!(item as syn::ExprClosure);
	item.capture = Some(syn::parse_quote!(move));
	let mut clown = Clown::default();
	clown.visit_expr_closure_mut(&mut item);

	let honks = clown.honks.into_iter().map(|(ident, expr)| { let span = expr.span(); quote_spanned! {span=> let #ident = (#expr).clone();} });
	let slips = clown.slips.into_iter().map(|(ident, expr)| { let span = expr.span(); quote_spanned! {span=> let #ident = #expr;} });

	quote! {{
		#(#honks)*
		#(#slips)*
		#item
	}}.into()
}
