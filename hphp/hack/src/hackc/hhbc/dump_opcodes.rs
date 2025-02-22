// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.

use anyhow::Result;
use hhbc::OpcodeData;
use quote::quote;
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn main() -> Result<()> {
    let mut opcode_data: Vec<OpcodeData> = hhbc::opcode_data().to_vec();
    opcode_data.sort_by(|a: &OpcodeData, b: &OpcodeData| a.name.cmp(b.name));

    let opcodes = emit_opcodes::emit_opcodes(
        quote!(
            enum Opcodes<'arena> {}
        ),
        &opcode_data,
    )?;

    let targets = emit_opcodes::emit_impl_targets(
        quote!(
            enum Opcodes<'arena> {}
        ),
        &opcode_data,
    )?;

    let output = format!("{}\n\n{}", opcodes, targets);

    let mut child = Command::new("rustfmt")
        .args(["--emit", "stdout"])
        .stdin(Stdio::piped())
        .spawn()?;
    child.stdin.as_mut().unwrap().write_all(output.as_bytes())?;
    child.wait()?;

    Ok(())
}
