use super::TripleSpace;
use crate::plan::barriers::NoBarrier;
use crate::plan::mutator_context::unreachable_release_func;
use crate::plan::mutator_context::Mutator;
use crate::plan::mutator_context::MutatorConfig;
use crate::plan::mutator_context::{
    create_allocator_mapping, create_space_mapping, ReservedAllocators,
};
use crate::plan::AllocationSemantics;
use crate::util::alloc::allocators::{AllocatorSelector, Allocators};
use crate::util::alloc::BumpAllocator;
use crate::util::VMMutatorThread;
use crate::util::VMThread;
use crate::util::VMWorkerThread;
use crate::vm::VMBinding;
use crate::MMTK;
use enum_map::{enum_map, EnumMap};

/// We use three bump allocators when enabling triplespace_multi_space.
const RESERVED_ALLOCATORS: ReservedAllocators = ReservedAllocators {
    n_bump_pointer: 1,
    ..ReservedAllocators::DEFAULT
};

lazy_static! {
    pub static ref ALLOCATOR_MAPPING: EnumMap<AllocationSemantics, AllocatorSelector> = {
        let mut map = create_allocator_mapping(RESERVED_ALLOCATORS, false);
        map[AllocationSemantics::Default] = AllocatorSelector::BumpPointer(0);
        map
    };
}

pub fn create_triplespace_mutator<VM: VMBinding>(
    mutator_tls: VMMutatorThread,
    mmtk: &'static MMTK<VM>,
) -> Mutator<VM> {
    let plan = mmtk.get_plan().downcast_ref::<TripleSpace<VM>>().unwrap();
    let config = MutatorConfig {
        allocator_mapping: &ALLOCATOR_MAPPING,
        space_mapping: Box::new({
            let mut vec = create_space_mapping(RESERVED_ALLOCATORS, true, plan);
            vec.push((AllocatorSelector::BumpPointer(0), plan.edenspace()));
            vec
        }),
        prepare_func: &triplespace_mutator_prepare,
        release_func: &triplespace_mutator_release,
    };

    Mutator {
        allocators: Allocators::<VM>::new(mutator_tls, mmtk, &config.space_mapping),
        barrier: Box::new(NoBarrier),
        mutator_tls,
        config,
        plan,
    }
}

pub fn triplespace_mutator_prepare<VM: VMBinding>(
    _mutator: &mut Mutator<VM>, _tls: VMWorkerThread) { }

pub fn triplespace_mutator_release<VM: VMBinding>(
    _mutator: &mut Mutator<VM>, _tls: VMWorkerThread
) {
    let bump_allocator = unsafe {
        _mutator
        .allocators
        .get_allocator_mut(_mutator.config.allocator_mapping[AllocationSemantics::Default])
    }
    .downcast_mut::<BumpAllocator<VM>>()
    .unwrap();

    bump_allocator.rebind(
        _mutator
            .plan
            .downcast_ref::<TripleSpace<VM>>()
            .unwrap()
            .edenspace()
    )
}
