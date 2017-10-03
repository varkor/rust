use traits;
use hir::def_id::DefId;
use ty::subst::Substs;
use ty::{self, Ty};
use syntax::ast::Mutability;
use hir::def::Def;
use hir::map as hir_map;

use super::{EvalResult, EvalContext, eval_context, MemoryPointer, Value, PrimVal,
            Machine, EvalErrorKind};

impl<'a, 'tcx, M: Machine<'tcx>> EvalContext<'a, 'tcx, M> {
    /// Creates a dynamic vtable for the given type and vtable origin. This is used only for
    /// objects.
    ///
    /// The `trait_ref` encodes the erased self type. Hence if we are
    /// making an object `Foo<Trait>` from a value of type `Foo<T>`, then
    /// `trait_ref` would map `T:Trait`.
    pub fn get_vtable(
        &mut self,
        ty: Ty<'tcx>,
        trait_ref: ty::PolyTraitRef<'tcx>,
    ) -> EvalResult<'tcx, MemoryPointer> {
        debug!("get_vtable(trait_ref={:?})", trait_ref);

        let size = self.type_size(trait_ref.self_ty())?.expect(
            "can't create a vtable for an unsized type",
        );
        let align = self.type_align(trait_ref.self_ty())?;

        let ptr_size = self.memory.pointer_size();
        let methods = ::traits::get_vtable_methods(self.tcx, trait_ref);
        let vtable = self.memory.allocate(
            ptr_size * (3 + methods.count() as u64),
            ptr_size,
            None,
        )?;

        let drop = eval_context::resolve_drop_in_place(self.tcx, ty);
        let drop = self.memory.create_fn_alloc(drop);
        self.memory.write_ptr_sized_unsigned(vtable, PrimVal::Ptr(drop))?;

        let size_ptr = vtable.offset(ptr_size, &self)?;
        self.memory.write_ptr_sized_unsigned(size_ptr, PrimVal::Bytes(size as u128))?;
        let align_ptr = vtable.offset(ptr_size * 2, &self)?;
        self.memory.write_ptr_sized_unsigned(align_ptr, PrimVal::Bytes(align as u128))?;

        for (i, method) in ::traits::get_vtable_methods(self.tcx, trait_ref).enumerate() {
            if let Some((def_id, substs)) = method {
                let instance = eval_context::resolve(self.tcx, def_id, substs);
                let fn_ptr = self.memory.create_fn_alloc(instance);
                let method_ptr = vtable.offset(ptr_size * (3 + i as u64), &self)?;
                self.memory.write_ptr_sized_unsigned(method_ptr, PrimVal::Ptr(fn_ptr))?;
            }
        }

        self.memory.mark_static_initalized(
            vtable.alloc_id,
            Mutability::Mutable,
        )?;

        Ok(vtable)
    }

    pub fn read_drop_type_from_vtable(
        &self,
        vtable: MemoryPointer,
    ) -> EvalResult<'tcx, Option<ty::Instance<'tcx>>> {
        // we don't care about the pointee type, we just want a pointer
        match self.read_ptr(vtable, self.tcx.mk_nil_ptr())? {
            // some values don't need to call a drop impl, so the value is null
            Value::ByVal(PrimVal::Bytes(0)) => Ok(None),
            Value::ByVal(PrimVal::Ptr(drop_fn)) => self.memory.get_fn(drop_fn).map(Some),
            _ => err!(ReadBytesAsPointer),
        }
    }

    pub fn read_size_and_align_from_vtable(
        &self,
        vtable: MemoryPointer,
    ) -> EvalResult<'tcx, (u64, u64)> {
        let pointer_size = self.memory.pointer_size();
        let size = self.memory.read_ptr_sized_unsigned(vtable.offset(pointer_size, self)?)?.to_bytes()? as u64;
        let align = self.memory.read_ptr_sized_unsigned(
            vtable.offset(pointer_size * 2, self)?
        )?.to_bytes()? as u64;
        Ok((size, align))
    }

    pub(crate) fn resolve_associated_const(
        &self,
        def_id: DefId,
        substs: &'tcx Substs<'tcx>,
    ) -> EvalResult<'tcx, ty::Instance<'tcx>> {
        match lookup_const_by_id(
            self.tcx,
            M::param_env(self).and((def_id, substs)),
        ) {
            Some((def_id, substs)) => Ok(ty::Instance::new(def_id, substs)),
            None => Err(EvalErrorKind::UnimplementedTraitSelection.into()),
        }
    }
}

/// * `DefId` is the id of the constant.
/// * `Substs` is the monomorphized substitutions for the expression.
fn lookup_const_by_id<'a, 'tcx>(tcx: ty::TyCtxt<'a, 'tcx, 'tcx>,
                                    key: ty::ParamEnvAnd<'tcx, (DefId, &'tcx Substs<'tcx>)>)
                                    -> Option<(DefId, &'tcx Substs<'tcx>)> {
    let (def_id, _) = key.value;
    if let Some(node_id) = tcx.hir.as_local_node_id(def_id) {
        match tcx.hir.find(node_id) {
            Some(hir_map::NodeTraitItem(_)) => {
                // If we have a trait item and the substitutions for it,
                // `resolve_trait_associated_const` will select an impl
                // or the default.
                resolve_trait_associated_const(tcx, key)
            }
            _ => Some(key.value)
        }
    } else {
        match tcx.describe_def(def_id) {
            Some(Def::AssociatedConst(_)) => {
                // As mentioned in the comments above for in-crate
                // constants, we only try to find the expression for a
                // trait-associated const if the caller gives us the
                // substitutions for the reference to it.
                if tcx.trait_of_item(def_id).is_some() {
                    resolve_trait_associated_const(tcx, key)
                } else {
                    Some(key.value)
                }
            }
            _ => Some(key.value)
        }
    }
}

fn resolve_trait_associated_const<'a, 'tcx>(tcx: ty::TyCtxt<'a, 'tcx, 'tcx>,
                                            key: ty::ParamEnvAnd<'tcx, (DefId, &'tcx Substs<'tcx>)>)
                                            -> Option<(DefId, &'tcx Substs<'tcx>)> {
    let param_env = key.param_env;
    let (def_id, substs) = key.value;
    let trait_item = tcx.associated_item(def_id);
    let trait_id = trait_item.container.id();
    let trait_ref = ty::Binder(ty::TraitRef::new(trait_id, substs));
    debug!("resolve_trait_associated_const: trait_ref={:?}",
           trait_ref);

    tcx.infer_ctxt().enter(|infcx| {
        let mut selcx = traits::SelectionContext::new(&infcx);
        let obligation = traits::Obligation::new(traits::ObligationCause::dummy(),
                                                 param_env,
                                                 trait_ref.to_poly_trait_predicate());
        let selection = match selcx.select(&obligation) {
            Ok(Some(vtable)) => vtable,
            // Still ambiguous, so give up and let the caller decide whether this
            // expression is really needed yet. Some associated constant values
            // can't be evaluated until monomorphization is done in trans.
            Ok(None) => {
                return None
            }
            Err(_) => {
                return None
            }
        };

        // NOTE: this code does not currently account for specialization, but when
        // it does so, it should hook into the param_env.reveal to determine when the
        // constant should resolve.
        match selection {
            traits::VtableImpl(ref impl_data) => {
                let name = trait_item.name;
                let ac = tcx.associated_items(impl_data.impl_def_id)
                    .find(|item| item.kind == ty::AssociatedKind::Const && item.name == name);
                match ac {
                    // FIXME(eddyb) Use proper Instance resolution to
                    // get the correct Substs returned from here.
                    Some(ic) => {
                        let substs = Substs::identity_for_item(tcx, ic.def_id);
                        Some((ic.def_id, substs))
                    }
                    None => {
                        if trait_item.defaultness.has_value() {
                            Some(key.value)
                        } else {
                            None
                        }
                    }
                }
            }
            traits::VtableParam(_) => None,
            _ => {
                bug!("resolve_trait_associated_const: unexpected vtable type {:?}", selection)
            }
        }
    })
}
