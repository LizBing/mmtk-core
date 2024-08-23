use crate::plan::global::BasePlan;
use crate::plan::global::CommonPlan;
use crate::plan::global::CreateGeneralPlanArgs;
use crate::plan::global::CreateSpecificPlanArgs;
use crate::plan::soda::mutator::ALLOCATOR_MAPPING;
use crate::plan::AllocationSemantics;
use crate::plan::Plan;
use crate::plan::PlanConstraints;
use crate::policy::copyspace::CopySpace;
use crate::policy::space::Space;
use crate::scheduler::*;
use crate::util::alloc::allocators::AllocatorSelector;
use crate::util::copy::*;
use crate::util::heap::gc_trigger::SpaceStats;
#[allow(unused_imports)]
use crate::util::heap::VMRequest;
use crate::util::metadata::side_metadata::SideMetadataContext;
use crate::util::opaque_pointer::*;
use crate::vm::VMBinding;
use enum_map::EnumMap;

use mmtk_macros::{HasSpaces, PlanTraceObject};

use std::sync::atomic::{Ordering, AtomicBool};

use super::gc_work::SodaWorkContext;

#[derive(HasSpaces, PlanTraceObject)]
pub struct Soda<VM: VMBinding> {
    pub hi: AtomicBool,

    #[parent]
    pub common: CommonPlan<VM>,

    #[space]
    #[copy_semantics(CopySemantics::DefaultCopy)]
    copy_space0: CopySpace<VM>,

    #[space]
    #[copy_semantics(CopySemantics::DefaultCopy)]
    copy_space1: CopySpace<VM>,
}

/// The plan constraints for the no gc plan.
pub const SODA_CONSTRAINTS: PlanConstraints = PlanConstraints {
    moves_objects: true,
    ..PlanConstraints::default()
};

impl<VM: VMBinding> Plan for Soda<VM> {
    fn constraints(&self) -> &'static PlanConstraints {
        &SODA_CONSTRAINTS
    }

    fn collection_required(&self, space_full: bool, _space: Option<SpaceStats<Self::VM>>) -> bool {
        self.base().collection_required(self, space_full)
    }

    fn common(&self) -> &CommonPlan<Self::VM> {
        &self.common
    }

    fn base(&self) -> &BasePlan<Self::VM> {
        &self.common.base
    }

    fn base_mut(&mut self) -> &mut BasePlan<Self::VM> {
        &mut self.common.base
    }

    fn prepare(&mut self, _tls: VMWorkerThread) {
        self.common.prepare(_tls, true);

        self.hi.store(
            !self.hi.load(Ordering::SeqCst),
            Ordering::SeqCst
        );

        let hi = self.hi.load(Ordering::SeqCst);
        self.copy_space0.prepare(hi);
        self.copy_space1.prepare(!hi);

        self.fromspace_mut().
            set_copy_for_sft_trace(Some(CopySemantics::DefaultCopy));
        self.tospace_mut().set_copy_for_sft_trace(None);
    }

    fn release(&mut self, _tls: VMWorkerThread) {
        self.common.release(_tls, true);
        self.fromspace().release();
    }

    fn prepare_worker(&self, _worker: &mut GCWorker<Self::VM>) {
        unsafe { _worker.get_copy_context_mut().copy[0].assume_init_mut() }
            .rebind(self.tospace())
    }

    fn get_allocator_mapping(&self) -> &'static EnumMap<AllocationSemantics, AllocatorSelector> {
        &ALLOCATOR_MAPPING
    }

    fn schedule_collection(&'static self, _scheduler: &GCWorkScheduler<VM>) {
        _scheduler.schedule_common_work::<SodaWorkContext<VM>>(self);
    }

    fn current_gc_may_move_object(&self) -> bool {
        true
    }

    fn get_used_pages(&self) -> usize {
        self.tospace().reserved_pages() + self.fromspace().reserved_pages()
    }

    fn get_collection_reserved_pages(&self) -> usize {
        self.tospace().reserved_pages()
    }

    fn create_copy_config(&'static self) -> CopyConfig<Self::VM> {
        use enum_map::enum_map;

        CopyConfig {
            copy_mapping: enum_map! {
                CopySemantics::DefaultCopy => CopySelector::CopySpace(0),
                _ => CopySelector::Unused
            },
            space_mapping: vec![
                (CopySelector::CopySpace(0), self.tospace())
            ],
            constraints: &SODA_CONSTRAINTS
        }
    }

}

impl<VM: VMBinding> Soda<VM> {
    pub fn new(args: CreateGeneralPlanArgs<VM>) -> Self {
        let mut plan_args = CreateSpecificPlanArgs {
            global_args: args,
            constraints: &SODA_CONSTRAINTS,
            global_side_metadata_specs: SideMetadataContext::new_global_specs(&[]),
        };

        let res = Soda {
            hi: AtomicBool::new(false),
            copy_space0: CopySpace::new(
                plan_args.get_space_args("copy_space0", true, false,
                                         VMRequest::discontiguous()), false),
            copy_space1: CopySpace::new(
                plan_args.get_space_args("copy_space1", true, false,
                                         VMRequest::discontiguous()), true),
            common: CommonPlan::new(plan_args)
        };

        res.verify_side_metadata_sanity();

        res
    }

    pub fn tospace(&self) -> &CopySpace<VM> {
        if self.hi.load(Ordering::SeqCst) {
            &self.copy_space1
        } else {
            &self.copy_space0
        }
    } 

    pub fn fromspace(&self) -> &CopySpace<VM> {
        if self.hi.load(Ordering::SeqCst) {
            &self.copy_space0
        } else {
            &self.copy_space1
        }
    }

    pub fn tospace_mut(&mut self) -> &mut CopySpace<VM> {
        if self.hi.load(Ordering::SeqCst) {
            &mut self.copy_space1
        } else {
            &mut self.copy_space0
        }
    }

    pub fn fromspace_mut(&mut self) -> &mut CopySpace<VM> {
        if self.hi.load(Ordering::SeqCst) {
            &mut self.copy_space0
        } else {
            &mut self.copy_space1
        }
    }
}
