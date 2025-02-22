// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.

use crate::decl_defs::{
    AbstractTypeconst, ClassConst, ConcreteTypeconst, DeclTy, DeclTy_, FunParam, FunType,
    PossiblyEnforcedTy, ShapeFieldType, TaccessType, Tparam, TypeConst, Typeconst, WhereConstraint,
};
use crate::reason::Reason;
use eq_modulo_pos::EqModuloPos;
use pos::{TypeName, TypeNameMap};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// note(sf, 2022-02-14): c.f. `Decl_subst`, `Decl_instantiate`

/// Maps type names to types with which to replace them.
#[derive(Debug, Clone, Eq, EqModuloPos, PartialEq, Serialize, Deserialize)]
#[serde(bound = "R: Reason")]
pub struct Subst<R: Reason>(pub TypeNameMap<DeclTy<R>>);

impl<R: Reason> From<TypeNameMap<DeclTy<R>>> for Subst<R> {
    fn from(map: TypeNameMap<DeclTy<R>>) -> Self {
        Self(map)
    }
}

impl<R: Reason> From<Subst<R>> for TypeNameMap<DeclTy<R>> {
    fn from(subst: Subst<R>) -> Self {
        subst.0
    }
}

#[derive(Debug, Clone)]
pub struct Substitution<'a, R: Reason> {
    pub subst: &'a Subst<R>,
}

impl<R: Reason> Subst<R> {
    pub fn new(tparams: &[Tparam<R, DeclTy<R>>], targs: &[DeclTy<R>]) -> Self {
        // If there are fewer type arguments than type parameters, we'll have
        // emitted an error elsewhere. We bind missing types to `Tany` (rather
        // than `Terr`) here to keep parity with the OCaml implementation, which
        // produces `Tany` because of a now-dead feature called "silent_mode".
        let targs = targs
            .iter()
            .cloned()
            .chain(std::iter::repeat(DeclTy::any(R::none())));
        Self(
            tparams
                .iter()
                .map(|tparam| tparam.name.id())
                .zip(targs)
                .collect(),
        )
    }
}

impl<'a, R: Reason> Substitution<'a, R> {
    fn merge_hk_type(
        &self,
        orig_r: R,
        orig_var: TypeName,
        ty: &DeclTy<R>,
        args: impl Iterator<Item = DeclTy<R>>,
    ) -> DeclTy<R> {
        let ty_: &DeclTy_<R> = ty.node();
        let res_ty_ = match ty_ {
            DeclTy_::DTapply(params) => {
                // We could insist on `existing_args.is_empty()` here
                // unless we want to support partial application.
                let (name, existing_args) = &**params;
                DeclTy_::DTapply(Box::new((
                    name.clone(),
                    existing_args.iter().cloned().chain(args).collect(),
                )))
            }
            DeclTy_::DTgeneric(params) => {
                // Same here.
                let (name, ref existing_args) = **params;
                DeclTy_::DTgeneric(Box::new((
                    name,
                    existing_args.iter().cloned().chain(args).collect(),
                )))
            }
            // We could insist on existing_args = [] here unless we want to
            // support partial application.
            _ => ty_.clone(),
        };
        let r = ty.reason().clone();
        DeclTy::new(R::instantiate(r, orig_var, orig_r), res_ty_)
    }

    pub fn instantiate(&self, ty: &DeclTy<R>) -> DeclTy<R> {
        // PERF: If subst is empty then instantiation is a no-op. We can save a
        // significant amount of CPU by avoiding recursively deconstructing the
        // `ty` data type.
        if self.subst.0.is_empty() {
            return ty.clone();
        }
        let r = ty.reason().clone();
        let ty_: &DeclTy_<R> = ty.node();
        match ty_ {
            DeclTy_::DTgeneric(params) => {
                let (x, ref existing_args) = **params;
                let args = existing_args.iter().map(|arg| self.instantiate(arg));
                match self.subst.0.get(&x) {
                    Some(x_ty) => self.merge_hk_type(r, x, x_ty, args),
                    None => DeclTy::generic(r, x, args.collect()),
                }
            }
            _ => DeclTy::new(r, self.instantiate_(ty_)),
        }
    }

    fn instantiate_(&self, x: &DeclTy_<R>) -> DeclTy_<R> {
        match x {
            DeclTy_::DTgeneric(_) => panic!("subst.rs: instantiate_: impossible!"),
            // IMPORTANT: We cannot expand `DTaccess` during instantiation
            // because this can be called before all type consts have been
            // declared and inherited.
            DeclTy_::DTaccess(ta) => DeclTy_::DTaccess(Box::new(TaccessType {
                ty: self.instantiate(&ta.ty),
                type_const: ta.type_const.clone(),
            })),
            DeclTy_::DTvecOrDict(tys) => DeclTy_::DTvecOrDict(Box::new((
                self.instantiate(&tys.0),
                self.instantiate(&tys.1),
            ))),
            DeclTy_::DTthis
            | DeclTy_::DTvar(_)
            | DeclTy_::DTmixed
            | DeclTy_::DTdynamic
            | DeclTy_::DTnonnull
            | DeclTy_::DTany
            | DeclTy_::DTerr
            | DeclTy_::DTprim(_) => x.clone(),
            DeclTy_::DTtuple(tys) => DeclTy_::DTtuple(
                tys.iter()
                    .map(|t| self.instantiate(t))
                    .collect::<Box<[_]>>(),
            ),
            DeclTy_::DTunion(tys) => DeclTy_::DTunion(
                tys.iter()
                    .map(|t| self.instantiate(t))
                    .collect::<Box<[_]>>(),
            ),
            DeclTy_::DTintersection(tys) => DeclTy_::DTintersection(
                tys.iter()
                    .map(|t| self.instantiate(t))
                    .collect::<Box<[_]>>(),
            ),
            DeclTy_::DToption(ty) => {
                let ty = self.instantiate(ty);
                // We want to avoid double option: `??T`.
                match ty.node() as &DeclTy_<R> {
                    ty_node @ DeclTy_::DToption(_) => ty_node.clone(),
                    _ => DeclTy_::DToption(ty),
                }
            }
            DeclTy_::DTlike(ty) => DeclTy_::DTlike(self.instantiate(ty)),
            DeclTy_::DTfun(ft) => {
                let tparams = &ft.tparams;
                let outer_subst = self;
                let mut subst = self.subst.clone();
                for tp in tparams.iter() {
                    subst.0.remove(tp.name.id_ref());
                }
                let subst = Substitution { subst: &subst };
                let params = ft
                    .params
                    .iter()
                    .map(|fp| FunParam {
                        ty: subst.instantiate_possibly_enforced_ty(&fp.ty),
                        pos: fp.pos.clone(),
                        name: fp.name,
                        flags: fp.flags,
                    })
                    .collect::<Box<[_]>>();
                let ret = subst.instantiate_possibly_enforced_ty(&ft.ret);
                let tparams = tparams
                    .iter()
                    .map(|tp| Tparam {
                        constraints: tp
                            .constraints
                            .iter()
                            .map(|(ck, ty)| (*ck, subst.instantiate(ty)))
                            .collect::<Box<[_]>>(),
                        variance: tp.variance,
                        name: tp.name.clone(),
                        tparams: tp.tparams.clone(),
                        reified: tp.reified,
                        user_attributes: tp.user_attributes.clone(),
                    })
                    .collect::<Box<[_]>>();
                let where_constraints = ft
                    .where_constraints
                    .iter()
                    .map(|WhereConstraint(ty1, ck, ty2)| {
                        WhereConstraint(subst.instantiate(ty1), *ck, outer_subst.instantiate(ty2))
                    })
                    .collect::<Box<[_]>>();
                DeclTy_::DTfun(Box::new(FunType {
                    params,
                    ret,
                    tparams,
                    where_constraints,
                    flags: ft.flags,
                    implicit_params: ft.implicit_params.clone(),
                    ifc_decl: ft.ifc_decl.clone(),
                }))
            }
            DeclTy_::DTapply(params) => {
                let (name, tys) = &**params;
                DeclTy_::DTapply(Box::new((
                    name.clone(),
                    tys.iter()
                        .map(|ty| self.instantiate(ty))
                        .collect::<Box<[_]>>(),
                )))
            }
            DeclTy_::DTshape(params) => {
                let (shape_kind, ref fdm) = **params;
                let fdm = fdm
                    .iter()
                    .map(|(f, sft)| {
                        (
                            *f,
                            ShapeFieldType {
                                field_name_pos: sft.field_name_pos.clone(),
                                ty: self.instantiate(&sft.ty),
                                optional: sft.optional,
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                DeclTy_::DTshape(Box::new((shape_kind, fdm)))
            }
        }
    }

    fn instantiate_possibly_enforced_ty(
        &self,
        et: &PossiblyEnforcedTy<DeclTy<R>>,
    ) -> PossiblyEnforcedTy<DeclTy<R>> {
        PossiblyEnforcedTy {
            ty: self.instantiate(&et.ty),
            enforced: et.enforced,
        }
    }

    pub fn instantiate_class_const(&self, cc: &ClassConst<R>) -> ClassConst<R> {
        ClassConst {
            is_synthesized: cc.is_synthesized,
            kind: cc.kind,
            pos: cc.pos.clone(),
            ty: self.instantiate(&cc.ty),
            origin: cc.origin,
            refs: cc.refs.clone(),
        }
    }

    fn instantiate_type_const_kind(&self, kind: &Typeconst<R>) -> Typeconst<R> {
        match kind {
            Typeconst::TCAbstract(k) => Typeconst::TCAbstract(AbstractTypeconst {
                as_constraint: k.as_constraint.as_ref().map(|ty| self.instantiate(ty)),
                super_constraint: k.super_constraint.as_ref().map(|ty| self.instantiate(ty)),
                default: k.default.as_ref().map(|ty| self.instantiate(ty)),
            }),
            Typeconst::TCConcrete(k) => Typeconst::TCConcrete(ConcreteTypeconst {
                ty: self.instantiate(&k.ty),
            }),
        }
    }

    pub fn instantiate_type_const(&self, tc: &TypeConst<R>) -> TypeConst<R> {
        TypeConst {
            is_synthesized: tc.is_synthesized,
            name: tc.name.clone(),
            kind: self.instantiate_type_const_kind(&tc.kind),
            origin: tc.origin,
            enforceable: tc.enforceable.clone(),
            reifiable: tc.reifiable.clone(),
            is_concretized: tc.is_concretized,
            is_ctx: tc.is_ctx,
        }
    }
}
