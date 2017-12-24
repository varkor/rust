// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rustc::mir;

use asm;
use builder::Builder;

use super::MirContext;
use super::LocalRef;

impl<'a, 'tcx> MirContext<'a, 'tcx> {
    pub fn trans_statement(&mut self,
                           bcx: Builder<'a, 'tcx>,
                           statement: &mir::Statement<'tcx>)
                           -> Builder<'a, 'tcx> {
        debug!("trans_statement(statement={:?})", statement);

        self.set_debug_loc(&bcx, statement.source_info);
        match statement.kind {
            mir::StatementKind::Assign(ref place, ref rvalue) => {
                if let mir::Place::Local(index) = *place {
                    match self.locals[index] {
                        LocalRef::Place(tr_dest, ind) => {
                            self.trans_rvalue(bcx, tr_dest, rvalue, ind)
                        }
                        LocalRef::Operand(None, ind) => {
                            let (bcx, operand) = self.trans_rvalue_operand(bcx, rvalue, ind);
                            self.locals[index] = LocalRef::Operand(Some(operand), ind);
                            bcx
                        }
                        LocalRef::Operand(Some(op), ind) => {
                            if !op.layout.is_zst() {
                                span_bug!(statement.source_info.span,
                                          "operand {:?} already assigned",
                                          rvalue);
                            }

                            // If the type is zero-sized, it's already been set here,
                            // but we still need to make sure we translate the operand
                            self.trans_rvalue_operand(bcx, rvalue, ind).0
                        }
                    }
                } else {
                    let tr_dest = self.trans_place(&bcx, place, 1001);
                    self.trans_rvalue(bcx, tr_dest, rvalue, 1001)
                }
            }
            mir::StatementKind::SetDiscriminant{ref place, variant_index} => {
                self.trans_place(&bcx, place, 1001)
                    .trans_set_discr(&bcx, variant_index);
                bcx
            }
            mir::StatementKind::StorageLive(local) => {
                if let LocalRef::Place(tr_place, _) = self.locals[local] {
                    tr_place.storage_live(&bcx);
                }
                bcx
            }
            mir::StatementKind::StorageDead(local) => {
                if let LocalRef::Place(tr_place, _) = self.locals[local] {
                    tr_place.storage_dead(&bcx);
                }
                bcx
            }
            mir::StatementKind::InlineAsm { ref asm, ref outputs, ref inputs } => {
                let outputs = outputs.iter().map(|output| {
                    self.trans_place(&bcx, output, 1001)
                }).collect();

                let input_vals = inputs.iter().map(|input| {
                    self.trans_operand(&bcx, input, 1001).immediate()
                }).collect();

                asm::trans_inline_asm(&bcx, asm, outputs, input_vals, 1001);
                bcx
            }
            mir::StatementKind::EndRegion(_) |
            mir::StatementKind::Validate(..) |
            mir::StatementKind::Nop => bcx,
        }
    }
}
