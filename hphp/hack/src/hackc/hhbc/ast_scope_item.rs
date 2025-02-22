// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.

use hhas_coeffects::HhasCoeffects;
use oxidized::{ast, file_info, pos::Pos};

use itertools::Either;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct LongLambda<'arena> {
    pub is_async: bool,
    pub coeffects: HhasCoeffects<'arena>,
}

#[derive(Clone, Debug)]
pub struct Lambda<'arena> {
    pub is_async: bool,
    pub coeffects: HhasCoeffects<'arena>,
}

#[derive(Clone, Debug)]
pub enum ScopeItem<'a, 'arena> {
    Class(Class<'a>),
    Function(Fun<'a>),
    Method(Method<'a>),
    LongLambda(LongLambda<'arena>),
    Lambda(Lambda<'arena>),
}

impl<'a, 'arena> ScopeItem<'a, 'arena> {
    pub fn is_in_lambda(&self) -> bool {
        matches!(self, ScopeItem::Lambda(_) | ScopeItem::LongLambda(_))
    }
}

#[derive(Debug)]
pub struct E<'a, AST, BRIEF>(Either<&'a AST, Rc<BRIEF>>);

impl<'a, AST, BRIEF> E<'a, AST, BRIEF> {
    pub fn new_ref(ast: &'a AST) -> Self {
        E(Either::Left(ast))
    }

    fn new_rc_(ast: &AST, f: impl Fn(&AST) -> BRIEF) -> Self {
        E(Either::Right(Rc::new(f(ast))))
    }

    fn either<'r, R: 'r>(
        &'r self,
        l: impl FnOnce(&'a AST) -> R,
        r: impl FnOnce(&'r BRIEF) -> R,
    ) -> R {
        match &self.0 {
            Either::Left(x) => l(x),
            Either::Right(x) => r(x.as_ref()),
        }
    }
}

impl<'a, AST, BRIEF> Clone for E<'a, AST, BRIEF> {
    fn clone(&self) -> Self {
        E(self.0.clone())
    }
}

pub type Class<'a> = E<'a, ast::Class_, Class_>;
pub type Fun<'a> = E<'a, ast::FunDef, Fun_>;
pub type Method<'a> = E<'a, ast::Method_, Method_>;

impl<'a> Class<'a> {
    pub fn new_rc(x: &ast::Class_) -> Self {
        Self::new_rc_(x, Class_::new)
    }

    pub(in crate) fn get_tparams(&self) -> &[ast::Tparam] {
        self.either(|x| &x.tparams[..], |x| &x.tparams[..])
    }

    pub fn get_span(&self) -> &Pos {
        self.either(|x| &x.span, |x| &x.span)
    }

    pub fn get_name(&self) -> &ast::Id {
        self.either(|x| &x.name, |x| &x.name)
    }

    pub fn get_name_str(&self) -> &str {
        &self.get_name().1
    }

    pub fn get_mode(&self) -> file_info::Mode {
        self.either(|x| x.mode, |x| x.mode)
    }

    pub fn get_kind(&self) -> ast::ClassishKind {
        self.either(|x| x.kind.clone(), |x| x.kind.clone())
    }

    pub fn get_extends(&self) -> &[ast::Hint] {
        self.either(|x| &x.extends[..], |x| &x.extends[..])
    }

    pub fn get_vars(&self) -> &[ast::ClassVar] {
        self.either(|x| &x.vars[..], |x| &x.vars[..])
    }
}

impl<'a> Fun<'a> {
    pub fn new_rc(x: &ast::FunDef) -> Self {
        Self::new_rc_(x, Fun_::new)
    }

    pub(in crate) fn get_tparams(&self) -> &[ast::Tparam] {
        self.either(
            |x: &'a ast::FunDef| &x.fun.tparams[..],
            |x: &Fun_| &x.tparams[..],
        )
    }

    pub(in crate) fn get_user_attributes(&self) -> &[ast::UserAttribute] {
        self.either(|x| &x.fun.user_attributes[..], |x| &x.user_attributes[..])
    }

    pub fn get_ctxs(&self) -> Option<&ast::Contexts> {
        self.either(|x| &x.fun.ctxs, |x| &x.ctxs).as_ref()
    }

    pub fn get_params(&self) -> &[ast::FunParam] {
        self.either(|x| &x.fun.params[..], |x| &x.params[..])
    }

    pub fn get_span(&self) -> &Pos {
        self.either(|x| &x.fun.span, |x| &x.span)
    }

    pub fn get_name(&self) -> &ast::Id {
        self.either(|x| &x.fun.name, |x| &x.name)
    }

    pub fn get_name_str(&self) -> &str {
        &self.get_name().1
    }

    pub fn get_mode(&self) -> file_info::Mode {
        self.either(|x| x.mode, |x| x.mode)
    }

    pub fn get_fun_kind(&self) -> ast::FunKind {
        self.either(|x| x.fun.fun_kind, |x| x.fun_kind)
    }
}

impl<'a> Method<'a> {
    pub fn new_rc(x: &ast::Method_) -> Self {
        Self::new_rc_(x, Method_::new)
    }

    pub(in crate) fn get_tparams(&self) -> &[ast::Tparam] {
        self.either(|x| &x.tparams[..], |x| &x.tparams[..])
    }

    pub(in crate) fn is_static(&self) -> bool {
        self.either(|x| x.static_, |x| x.static_)
    }

    pub(in crate) fn get_user_attributes(&self) -> &[ast::UserAttribute] {
        self.either(|x| &x.user_attributes[..], |x| &x.user_attributes[..])
    }

    pub fn get_ctxs(&self) -> Option<&ast::Contexts> {
        self.either(|x| &x.ctxs, |x| &x.ctxs).as_ref()
    }

    pub fn get_params(&self) -> &[ast::FunParam] {
        self.either(|x| &x.params[..], |x| &x.params[..])
    }

    pub fn get_span(&self) -> &Pos {
        self.either(|x| &x.span, |x| &x.span)
    }

    pub fn get_name(&self) -> &ast::Id {
        self.either(|x| &x.name, |x| &x.name)
    }

    pub fn get_name_str(&self) -> &str {
        &self.get_name().1
    }

    pub fn get_fun_kind(&self) -> ast::FunKind {
        self.either(|x| x.fun_kind, |x| x.fun_kind)
    }
}

#[derive(Debug)]
pub struct Class_ {
    name: ast::Id,
    span: Pos,
    tparams: Vec<ast::Tparam>,
    vars: Vec<ast::ClassVar>,
    mode: file_info::Mode,
    kind: ast::ClassishKind,
    extends: Vec<ast::Hint>,
}

impl Class_ {
    fn new(c: &ast::Class_) -> Self {
        Self {
            name: c.name.clone(),
            span: c.span.clone(),
            tparams: c.tparams.clone(),
            vars: c.vars.clone(),
            mode: c.mode,
            kind: c.kind.clone(),
            extends: c.extends.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Fun_ {
    name: ast::Id,
    span: Pos,
    tparams: Vec<ast::Tparam>,
    user_attributes: Vec<ast::UserAttribute>,
    mode: file_info::Mode,
    fun_kind: ast::FunKind,
    ctxs: Option<ast::Contexts>,
    params: Vec<ast::FunParam>,
}

impl Fun_ {
    fn new(fd: &ast::FunDef) -> Self {
        let f = &fd.fun;
        Self {
            name: f.name.clone(),
            span: f.span.clone(),
            tparams: f.tparams.clone(),
            user_attributes: f.user_attributes.clone(),
            mode: fd.mode,
            fun_kind: f.fun_kind,
            ctxs: f.ctxs.clone(),
            params: f.params.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Method_ {
    name: ast::Id,
    span: Pos,
    tparams: Vec<ast::Tparam>,
    user_attributes: Vec<ast::UserAttribute>,
    static_: bool,
    fun_kind: ast::FunKind,
    ctxs: Option<ast::Contexts>,
    params: Vec<ast::FunParam>,
}

impl Method_ {
    fn new(m: &ast::Method_) -> Self {
        Self {
            name: m.name.clone(),
            span: m.span.clone(),
            tparams: m.tparams.clone(),
            static_: m.static_,
            user_attributes: m.user_attributes.clone(),
            fun_kind: m.fun_kind,
            ctxs: m.ctxs.clone(),
            params: m.params.clone(),
        }
    }
}
