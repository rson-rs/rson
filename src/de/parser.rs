use std::collections::{HashMap, HashSet};

use proc_macro2::{Spacing, TokenStream, TokenTree};
use syn::parse::Parser;
use syn::{Expr, ExprCall, ExprPath, Member, Pat, Stmt};

use super::error::RsonDeResult;
use super::RsonDeserializer;
use crate::de::error::RsonDeError;

pub struct RsonParser {
    pub(super) vars: HashMap<String, Box<Expr>>,
    pub(super) return_expr: Option<Expr>,
}

impl RsonParser {
    pub fn from_str(input: &str) -> RsonDeResult<Self> {
        let statements = syn::Block::parse_within.parse_str(input)?;
        Self::new(statements)
    }

    pub fn new(mut statements: Vec<Stmt>) -> RsonDeResult<Self> {
        let mut return_expr = None;

        if let Some(stmt) = statements.pop() {
            if let Stmt::Expr(current_expr, None) = stmt {
                return_expr = Some(current_expr);
            } else {
                statements.push(stmt);
            }
        }

        let mut vars = HashMap::new();

        for stmt in statements {
            match stmt {
                Stmt::Local(local) => {
                    if let (Pat::Ident(ident), Some(init)) = (local.pat, local.init) {
                        vars.insert(ident.ident.to_string(), init.expr);
                    }
                },
                _ => (),
            }
        }

        Ok(Self { vars, return_expr })
    }

    pub fn new_deserializer(&self) -> RsonDeResult<RsonDeserializer> {
        RsonDeserializer::new(self)
    }

    pub fn is_empty(&self) -> bool {
        let Self { vars, return_expr } = self;
        vars.is_empty() && return_expr.is_none()
    }

    pub fn find_field_init_expr<'a>(&'a self, base: &'a Expr, member: &Member) -> RsonDeResult<&'a Expr> {
        let mut state = FindInitState::default();
        self.find_field_init_expr_inner(base, member, &mut state, false)
    }

    fn find_field_init_expr_inner<'a>(
        &'a self,
        base: &'a Expr,
        member: &Member,
        state: &mut FindInitState,
        nested: bool,
    ) -> RsonDeResult<&'a Expr> {
        let mut base_init = match base {
            Expr::Field(field) => self.find_field_init_expr_inner(&field.base, &field.member, state, true)?,
            Expr::Path(path) => self.find_var_init_expr_inner(base, &path.path, state, nested)?,
            _ => {
                return Err(RsonDeError::Parse(format!(
                    "Invalid base expr {base:?} for member {member:?}"
                )))
            },
        };

        loop {
            match base_init {
                Expr::Field(_) => (),
                Expr::Path(path) if path.path.segments.len() > 1 || is_expr_path_none(base_init) => {
                    return Ok(base_init);
                },
                Expr::Path(_) => (),
                _ => break,
            }

            match base_init {
                Expr::Field(field) => {
                    base_init = self.find_field_init_expr_inner(&field.base, &field.member, state, true)?;
                },
                Expr::Path(path) => {
                    base_init = self.find_var_init_expr_inner(base_init, &path.path, state, true)?;
                },
                _ => unreachable!(),
            }
        }

        match base_init {
            Expr::Struct(struct_expr) => {
                let Member::Named(ident) = member else {
                    return Err(RsonDeError::Parse(format!(
                        "Invalid member {member:?} for struct {struct_expr:?}"
                    )));
                };
                struct_expr
                    .fields
                    .iter()
                    .find(|field| &field.member == member)
                    .map(|field| &field.expr)
                    .ok_or_else(|| RsonDeError::Parse(format!("Field `{ident}` for struct {struct_expr:?} not found")))
            },
            Expr::Tuple(tuple) => {
                let Member::Unnamed(index) = member else {
                    return Err(RsonDeError::Parse(format!(
                        "Invalid member {member:?} for tuple {tuple:?}"
                    )));
                };
                let index = index.index as usize;

                tuple
                    .elems
                    .get(index)
                    .ok_or_else(|| RsonDeError::Parse(format!("Index `{index}` for tuple {tuple:?} not found")))
            },
            _ => Err(RsonDeError::Parse(format!(
                "Invalid base initialization expr {base_init:?} for member {member:?}"
            ))),
        }
    }

    pub fn find_var_init_expr<'a>(&'a self, source: &'a Expr, path: &syn::Path) -> RsonDeResult<&'a Expr> {
        let mut state = FindInitState::default();
        self.find_var_init_expr_inner(source, path, &mut state, false)
    }

    fn find_var_init_expr_inner<'a>(
        &'a self,
        source: &'a Expr,
        path: &syn::Path,
        state: &mut FindInitState,
        nested: bool,
    ) -> RsonDeResult<&'a Expr> {
        let Some(name) = path.get_ident().map(|ident| ident.to_string()) else {
            // Not a var
            return Ok(source);
        };

        let Some(init) = self.vars.get(&name) else {
            // Not a var
            return Ok(source);
        };

        if !nested {
            if state.watched_names.contains(&name) {
                return Err(RsonDeError::Parse(format!("Recursive var `{name}`: {state:?}")));
            }
            state.watched_names.insert(name);
        }

        let init = init.as_ref();
        match init {
            Expr::Field(field) => self.find_field_init_expr_inner(&field.base, &field.member, state, false),
            Expr::Path(path) => self.find_var_init_expr_inner(source, &path.path, state, false),
            _ => Ok(init),
        }
    }
}

#[derive(Debug, Default)]
pub struct FindInitState {
    pub watched_names: HashSet<String>,
}

pub fn is_expr_path_none(expr: &Expr) -> bool {
    if let Expr::Path(ExprPath { path, .. }) = expr {
        path.get_ident()
            .map(|ident| ident.to_string() == "None")
            .unwrap_or(false)
    } else {
        false
    }
}

pub fn expected_func_call_args(expr: &Expr, name: impl AsRef<str>, args_count: usize) -> Option<Vec<&Expr>> {
    if let Expr::Call(ExprCall { func, args, .. }) = expr {
        if let Expr::Path(ExprPath { path, .. }) = func.as_ref() {
            let name = name.as_ref();

            if (name == "" // Allow for enum tuple struct variant
                || path
                    .get_ident()
                    .map(|ident| ident.to_string() == name)
                    .unwrap_or(false))
                && args.len() == args_count
            {
                return Some(args.iter().collect());
            }
        }
    }
    None
}

pub fn make_tuples_from_map_macro(tokens: TokenStream) -> String {
    let mut map_tuples = String::from("(");
    let mut separate_value = true;
    let mut value = String::new();

    for token in tokens {
        if let TokenTree::Punct(punct) = &token {
            match (punct.as_char(), punct.spacing()) {
                (':', Spacing::Joint) => {
                    separate_value = false;
                },
                (':', Spacing::Alone) => {
                    if separate_value {
                        map_tuples.push('(');
                        map_tuples.push_str(&value);
                        map_tuples.push(',');

                        value = String::new();
                        continue;
                    } else {
                        separate_value = true;
                    }
                },
                (',', Spacing::Alone) => {
                    map_tuples.push_str(&value);
                    map_tuples.push(')');
                    map_tuples.push(',');

                    value = String::new();
                    separate_value = true;
                    continue;
                },
                _ => {},
            }
        }
        value.push_str(&token.to_string());
    }

    if value.is_empty() && map_tuples.ends_with("),") {
        map_tuples.pop();
    } else {
        map_tuples.push_str(&value);
        map_tuples.push(')');
    }
    map_tuples.push(')');

    map_tuples
}
