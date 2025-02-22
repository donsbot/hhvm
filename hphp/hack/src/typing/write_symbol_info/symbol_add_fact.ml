(*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the "hack" directory of this source tree.
 *
 *)

open Aast
open Hh_prelude
open Hh_json
module Util = Symbol_json_util
module Build_json = Symbol_build_json
module Predicate = Symbol_predicate
module Fact_id = Symbol_fact_id
module Fact_acc = Symbol_predicate.Fact_acc

let namespace_decl name progress =
  let json_fields =
    [("name", Build_json.build_namespaceqname_json_nested name)]
  in
  Fact_acc.add_fact
    Predicate.(Hack NamespaceDeclaration)
    (JSON_Object json_fields)
    progress

(* create a namespace fact from a namespace attribute in node. Name can be empty
   in that case we simply ignore it *)
let namespace_decl_opt name progress =
  match name.Namespace_env.ns_name with
  | None -> progress (* global name space *)
  | Some "" -> progress
  | Some name -> namespace_decl name progress |> snd

let container_decl decl_pred name progress =
  let json = JSON_Object [("name", Build_json.build_qname_json_nested name)] in
  Fact_acc.add_fact decl_pred json progress

(* Helper function for adding facts for container parents, given
a context, a list of declarations, a predicate type, and progress state *)
let parent_decls ctx decls pred prog =
  List.fold decls ~init:([], prog) ~f:(fun (decl_refs, prog) decl ->
      let name = Util.strip_tparams (Util.get_type_from_hint ctx decl) in
      let (decl_id, prog) = container_decl pred name prog in
      let ref = Build_json.build_id_json decl_id in
      (ref :: decl_refs, prog))

let container_defn ctx source_text clss decl_id member_decls prog =
  let prog = namespace_decl_opt clss.c_namespace prog in
  let tparams =
    List.map
      clss.c_tparams
      ~f:(Build_json.build_type_param_json ctx source_text)
  in
  let common_fields =
    [
      ("declaration", Build_json.build_id_json decl_id);
      ("members", JSON_Array member_decls);
      ( "attributes",
        Build_json.build_attributes_json_nested
          source_text
          clss.c_user_attributes );
      ("typeParams", JSON_Array tparams);
    ]
  in
  let (req_extends_hints, req_implements_hints) =
    Aast.partition_map_require_kind ~f:(fun x -> x) clss.c_reqs
  in
  let (req_extends, prog) =
    parent_decls
      ctx
      (List.map req_extends_hints ~f:fst)
      Predicate.(Hack ClassDeclaration)
      prog
  in
  let (req_implements, prog) =
    parent_decls
      ctx
      (List.map req_implements_hints ~f:fst)
      Predicate.(Hack InterfaceDeclaration)
      prog
  in
  let (defn_pred, json_fields, prog) =
    match Predicate.get_parent_kind clss with
    | Predicate.InterfaceContainer ->
      let (extends, prog) =
        parent_decls
          ctx
          clss.c_extends
          Predicate.(Hack InterfaceDeclaration)
          prog
      in
      let req_fields =
        common_fields
        @ [
            ("extends_", JSON_Array extends);
            ("requireExtends", JSON_Array req_extends);
          ]
      in
      (Predicate.(Hack InterfaceDefinition), req_fields, prog)
    | Predicate.TraitContainer ->
      let (impls, prog) =
        parent_decls
          ctx
          clss.c_implements
          Predicate.(Hack InterfaceDeclaration)
          prog
      in
      let (uses, prog) =
        parent_decls ctx clss.c_uses Predicate.(Hack TraitDeclaration) prog
      in
      let req_fields =
        common_fields
        @ [
            ("implements_", JSON_Array impls);
            ("uses", JSON_Array uses);
            ("requireExtends", JSON_Array req_extends);
            ("requireImplements", JSON_Array req_implements);
          ]
      in
      (Predicate.(Hack TraitDefinition), req_fields, prog)
    | Predicate.ClassContainer ->
      let is_abstract = Ast_defs.is_classish_abstract clss.c_kind in
      let (class_fields, prog) =
        let (impls, prog) =
          parent_decls
            ctx
            clss.c_implements
            Predicate.(Hack InterfaceDeclaration)
            prog
        in
        let (uses, prog) =
          parent_decls ctx clss.c_uses Predicate.(Hack TraitDeclaration) prog
        in
        let req_class_fields =
          common_fields
          @ Hh_json.
              [
                ("isAbstract", JSON_Bool is_abstract);
                ("isFinal", JSON_Bool clss.c_final);
                ("implements_", JSON_Array impls);
                ("uses", JSON_Array uses);
              ]
        in
        match clss.c_extends with
        | [] -> (req_class_fields, prog)
        | [parent] ->
          let (decl_id, prog) =
            let parent_clss =
              Util.strip_tparams (Util.get_type_from_hint ctx parent)
            in
            container_decl Predicate.(Hack ClassDeclaration) parent_clss prog
          in
          ( ("extends_", Build_json.build_id_json decl_id) :: req_class_fields,
            prog )
        | _ ->
          Hh_logger.log
            "WARNING: skipping extends field for class with multiple parents %s"
            (snd clss.c_name);
          (req_class_fields, prog)
      in
      (Predicate.(Hack ClassDefinition), class_fields, prog)
  in
  Fact_acc.add_fact defn_pred (JSON_Object json_fields) prog

let property_decl con_type decl_id name progress =
  let json =
    JSON_Object
      [
        ("name", Build_json.build_name_json_nested name);
        ("container", Build_json.build_container_json_ref con_type decl_id);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack PropertyDeclaration) json progress

let class_const_decl con_type decl_id name progress =
  let json =
    JSON_Object
      [
        ("name", Build_json.build_name_json_nested name);
        ("container", Build_json.build_container_json_ref con_type decl_id);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack ClassConstDeclaration) json progress

let type_const_decl con_type decl_id name progress =
  let json =
    JSON_Object
      [
        ("name", Build_json.build_name_json_nested name);
        ("container", Build_json.build_container_json_ref con_type decl_id);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack TypeConstDeclaration) json progress

let method_decl con_type decl_id name progress =
  let json =
    JSON_Object
      [
        ("name", Build_json.build_name_json_nested name);
        ("container", Build_json.build_container_json_ref con_type decl_id);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack MethodDeclaration) json progress

let method_defn ctx source_text meth decl_id progress =
  let tparams =
    List.map
      meth.m_tparams
      ~f:(Build_json.build_type_param_json ctx source_text)
  in
  let json =
    JSON_Object
      [
        ("declaration", Build_json.build_id_json decl_id);
        ( "signature",
          Build_json.build_signature_json
            ctx
            source_text
            meth.m_params
            meth.m_ctxs
            meth.m_ret );
        ("visibility", Build_json.build_visibility_json meth.m_visibility);
        ("isAbstract", JSON_Bool meth.m_abstract);
        ("isAsync", Build_json.build_is_async_json meth.m_fun_kind);
        ("isFinal", JSON_Bool meth.m_final);
        ("isStatic", JSON_Bool meth.m_static);
        ( "attributes",
          Build_json.build_attributes_json_nested
            source_text
            meth.m_user_attributes );
        ("typeParams", JSON_Array tparams);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack MethodDefinition) json progress

let method_overrides
    meth_name base_cont_name base_cont_type der_cont_name der_cont_type prog =
  let json =
    JSON_Object
      [
        ( "derived",
          Build_json.build_method_decl_nested
            meth_name
            der_cont_name
            der_cont_type );
        ( "base",
          Build_json.build_method_decl_nested
            meth_name
            base_cont_name
            base_cont_type );
      ]
  in
  Fact_acc.add_fact Predicate.(Hack MethodOverrides) json prog

let property_defn ctx source_text prop decl_id progress =
  let base_fields =
    [
      ("declaration", Build_json.build_id_json decl_id);
      ("visibility", Build_json.build_visibility_json prop.cv_visibility);
      ("isFinal", JSON_Bool prop.cv_final);
      ("isAbstract", JSON_Bool prop.cv_abstract);
      ("isStatic", JSON_Bool prop.cv_is_static);
      ( "attributes",
        Build_json.build_attributes_json_nested
          source_text
          prop.cv_user_attributes );
    ]
  in
  let json_fields =
    match hint_of_type_hint prop.cv_type with
    | None -> base_fields
    | Some h ->
      let ty = Util.get_type_from_hint ctx h in
      ("type", Build_json.build_type_json_nested ty) :: base_fields
  in
  Fact_acc.add_fact
    Predicate.(Hack PropertyDefinition)
    (JSON_Object json_fields)
    progress

let class_const_defn ctx source_text const decl_id progress =
  let base_fields = [("declaration", Build_json.build_id_json decl_id)] in
  let json_fields =
    match const.cc_kind with
    | CCAbstract None -> base_fields
    | CCAbstract (Some expr)
    | CCConcrete expr ->
      let value = Util.ast_expr_to_json source_text expr in
      ("value", value) :: base_fields
  in
  let json_fields =
    match const.cc_type with
    | None -> json_fields
    | Some h ->
      let ty = Util.get_type_from_hint ctx h in
      ("type", Build_json.build_type_json_nested ty) :: json_fields
  in
  Fact_acc.add_fact
    Predicate.(Hack ClassConstDefinition)
    (JSON_Object json_fields)
    progress

let type_const_defn ctx source_text tc decl_id progress =
  let base_fields =
    [
      ("declaration", Build_json.build_id_json decl_id);
      ("kind", Build_json.build_type_const_kind_json tc.c_tconst_kind);
      ( "attributes",
        Build_json.build_attributes_json_nested
          source_text
          tc.c_tconst_user_attributes );
    ]
  in
  let json_fields =
    (* TODO(T88552052) should the default of an abstract type constant be used
     * as a value here *)
    match tc.c_tconst_kind with
    | TCConcrete { c_tc_type = h }
    | TCAbstract { c_atc_default = Some h; _ } ->
      let ty = Util.get_type_from_hint ctx h in
      ("type", Build_json.build_type_json_nested ty) :: base_fields
    | TCAbstract { c_atc_default = None; _ } -> base_fields
  in
  Fact_acc.add_fact
    Predicate.(Hack TypeConstDefinition)
    (JSON_Object json_fields)
    progress

let enum_decl name progress =
  let json = JSON_Object [("name", Build_json.build_qname_json_nested name)] in
  Fact_acc.add_fact Predicate.(Hack EnumDeclaration) json progress

let enum_defn ctx source_text enm enum_id enum_data enumerators progress =
  let prog = namespace_decl_opt enm.c_namespace progress in
  let (includes, prog) =
    parent_decls ctx enum_data.e_includes Predicate.(Hack EnumDeclaration) prog
  in
  let is_enum_class = Aast.is_enum_class enm in
  let json_fields =
    [
      ("declaration", Build_json.build_id_json enum_id);
      ( "enumBase",
        Build_json.build_type_json_nested
          (Util.get_type_from_hint ctx enum_data.e_base) );
      ("enumerators", JSON_Array enumerators);
      ( "attributes",
        Build_json.build_attributes_json_nested
          source_text
          enm.c_user_attributes );
      ("includes", JSON_Array includes);
      ("isEnumClass", JSON_Bool is_enum_class);
    ]
  in
  let json_fields =
    match enum_data.e_constraint with
    | None -> json_fields
    | Some c ->
      ( "enumConstraint",
        Build_json.build_type_json_nested (Util.get_type_from_hint ctx c) )
      :: json_fields
  in
  Fact_acc.add_fact
    Predicate.(Hack EnumDefinition)
    (JSON_Object json_fields)
    prog

let enumerator decl_id const_name progress =
  let json =
    JSON_Object
      [
        ("name", Build_json.build_name_json_nested const_name);
        ("enumeration", Build_json.build_id_json decl_id);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack Enumerator) json progress

let func_decl name progress =
  let json = JSON_Object [("name", Build_json.build_qname_json_nested name)] in
  Fact_acc.add_fact Predicate.(Hack FunctionDeclaration) json progress

let func_defn ctx source_text fd decl_id progress =
  let elem = fd.fd_fun in
  let prog = namespace_decl_opt fd.fd_namespace progress in
  let tparams =
    List.map
      elem.f_tparams
      ~f:(Build_json.build_type_param_json ctx source_text)
  in
  let json_fields =
    [
      ("declaration", Build_json.build_id_json decl_id);
      ( "signature",
        Build_json.build_signature_json
          ctx
          source_text
          elem.f_params
          elem.f_ctxs
          elem.f_ret );
      ("isAsync", Build_json.build_is_async_json elem.f_fun_kind);
      ( "attributes",
        Build_json.build_attributes_json_nested
          source_text
          elem.f_user_attributes );
      ("typeParams", JSON_Array tparams);
    ]
  in
  Fact_acc.add_fact
    Predicate.(Hack FunctionDefinition)
    (JSON_Object json_fields)
    prog

let typedef_decl name progress =
  let json = JSON_Object [("name", Build_json.build_qname_json_nested name)] in
  Fact_acc.add_fact Predicate.(Hack TypedefDeclaration) json progress

let typedef_defn ctx source_text elem decl_id progress =
  let prog = namespace_decl_opt elem.t_namespace progress in
  let is_transparent =
    match elem.t_vis with
    | Transparent -> true
    | Tinternal -> true
    | Opaque -> false
  in
  let tparams =
    List.map
      elem.t_tparams
      ~f:(Build_json.build_type_param_json ctx source_text)
  in
  let json_fields =
    [
      ("declaration", Build_json.build_id_json decl_id);
      ("isTransparent", JSON_Bool is_transparent);
      ( "attributes",
        Build_json.build_attributes_json_nested
          source_text
          elem.t_user_attributes );
      ("typeParams", JSON_Array tparams);
    ]
  in
  Fact_acc.add_fact
    Predicate.(Hack TypedefDefinition)
    (JSON_Object json_fields)
    prog

let gconst_decl name progress =
  let json = JSON_Object [("name", Build_json.build_qname_json_nested name)] in
  Fact_acc.add_fact Predicate.(Hack GlobalConstDeclaration) json progress

let gconst_defn ctx source_text elem decl_id progress =
  let prog = namespace_decl_opt elem.cst_namespace progress in
  let value = Util.ast_expr_to_json source_text elem.cst_value in
  let req_fields =
    [("declaration", Build_json.build_id_json decl_id); ("value", value)]
  in
  let json_fields =
    match elem.cst_type with
    | None -> req_fields
    | Some h ->
      let ty = Util.get_type_from_hint ctx h in
      ("type", Build_json.build_type_json_nested ty) :: req_fields
  in
  let json = JSON_Object json_fields in
  Fact_acc.add_fact Predicate.(Hack GlobalConstDefinition) json prog

let decl_loc pos decl_json progress =
  let filepath = Relative_path.to_absolute (Pos.filename pos) in
  let json =
    JSON_Object
      [
        ("declaration", decl_json);
        ("file", Build_json.build_file_json_nested filepath);
        ("span", Build_json.build_bytespan_json pos);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack DeclarationLocation) json progress

let decl_comment pos decl_json progress =
  let filepath = Relative_path.to_absolute (Pos.filename pos) in
  let json =
    JSON_Object
      [
        ("declaration", decl_json);
        ("file", Build_json.build_file_json_nested filepath);
        ("span", Build_json.build_bytespan_json pos);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack DeclarationComment) json progress

let decl_span pos decl_json progress =
  let filepath = Relative_path.to_absolute (Pos.filename pos) in
  let json =
    JSON_Object
      [
        ("declaration", decl_json);
        ("file", Build_json.build_file_json_nested filepath);
        ("span", Build_json.build_bytespan_json pos);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack DeclarationSpan) json progress

let file_lines filepath sourceText progress =
  let lineLengths =
    Line_break_map.offsets_to_line_lengths
      sourceText.Full_fidelity_source_text.offset_map
  in
  let endsInNewline = Util.ends_in_newline sourceText in
  let hasUnicodeOrTabs = Util.has_tabs_or_multibyte_codepoints sourceText in
  let json =
    Build_json.build_file_lines_json
      filepath
      lineLengths
      endsInNewline
      hasUnicodeOrTabs
  in
  Fact_acc.add_fact Predicate.(Src FileLines) json progress

let file_xrefs filepath xref_map progress =
  let json =
    JSON_Object
      [
        ("file", Build_json.build_file_json_nested filepath);
        ("xrefs", Build_json.build_xrefs_json xref_map);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack FileXRefs) json progress

let file_decls filepath decls progress =
  let json =
    JSON_Object
      [
        ("file", Build_json.build_file_json_nested filepath);
        ("declarations", JSON_Array decls);
      ]
  in
  Fact_acc.add_fact Predicate.(Hack FileDeclarations) json progress

let method_occ receiver_class name progress =
  let module SO = SymbolOccurrence in
  let json =
    List.concat
    @@ [
         [("name", Build_json.build_name_json_nested name)];
         (match receiver_class with
         | SO.ClassName className ->
           [("className", Build_json.build_name_json_nested className)]
         | SO.UnknownClass -> []);
       ]
  in
  Fact_acc.add_fact
    Predicate.(Hack MethodOccurrence)
    (JSON_Object json)
    progress
