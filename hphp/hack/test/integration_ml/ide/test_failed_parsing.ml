(*
 * Copyright (c) 2016, Facebook, Inc.
 * All rights reserved.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the "hack" directory of this source tree.
 *
 *
 *)

module Test = Integration_test_base

let foo_name = "foo.php"

let foo_contents =
  Printf.sprintf "<?hh // strict
/* HH_FIXME[4336] */
function foo(): %s {

}"

let foo_returns_int_contents = foo_contents "int"

let foo_returns_string_contents = foo_contents "string"

let bar_name = "bar.php"

let bar_contents = "<?hh // strict

function bar() : void {
  foo();
}"

let parse_error = "<?hh

{"

let bar_parse_error_diagnostics =
  "
/bar.php:
File \"/bar.php\", line 3, characters 1-1:
Hack does not support top level statements. Use the `__EntryPoint` attribute on a function instead (Parsing[1002])

File \"/bar.php\", line 3, characters 2-2:
A right brace `}` is expected here. (Parsing[1002])
"

let root = "/"

let hhconfig_filename = Filename.concat root ".hhconfig"

let hhconfig_contents =
  "
allowed_fixme_codes_strict = 4336
allowed_decl_fixme_codes = 4336
"

let test () =
  Relative_path.set_path_prefix Relative_path.Root (Path.make root);
  TestDisk.set hhconfig_filename hhconfig_contents;
  let hhconfig_path =
    Relative_path.create Relative_path.Root hhconfig_filename
  in
  let options = ServerArgs.default_options ~root in
  let (custom_config, _) =
    ServerConfig.load ~silent:false hhconfig_path options
  in
  let env = Test.setup_server ~custom_config () in
  let env =
    Test.setup_disk
      env
      [(foo_name, foo_returns_int_contents); (bar_name, bar_contents)]
  in
  let env = Test.connect_persistent_client env in
  Test.assert_no_errors env;

  let env = Test.wait env in
  let (env, loop_output) = Test.(run_loop_once env default_loop_input) in
  Test.assert_no_diagnostics loop_output;

  let env = Test.open_file env bar_name ~contents:parse_error in
  let env = Test.wait env in
  let (env, loop_output) = Test.(run_loop_once env default_loop_input) in
  Test.assert_diagnostics_string loop_output bar_parse_error_diagnostics;

  let env = Test.open_file env foo_name ~contents:foo_returns_string_contents in
  let env = Test.wait env in
  let (_, loop_output) = Test.(run_loop_once env default_loop_input) in
  (* Bar depends on foo, so change of foo will trigger recheck of bar.
   * Verify that doing this doesn't change previous parsing stage errors *)
  Test.assert_no_diagnostics loop_output
