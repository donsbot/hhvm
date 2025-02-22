(*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the "hack" directory of this source tree.
 *
 *)

(** A program analysis to find shape like dicts and the static keys used in
    these dicts. *)

open Shape_analysis_types

exception Shape_analysis_exn of string

val simplify : Typing_env_types.env -> constraint_ list -> shape_result list

val callable :
  Tast_env.t -> Tast.fun_param list -> Tast.func_body -> constraint_ list

val do_ : options -> Provider_context.t -> Tast.program -> unit

val show_shape_result : Typing_env_types.env -> shape_result -> string
