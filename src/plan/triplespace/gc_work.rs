use super::TripleSpace;
use crate::scheduler::gc_work::{SFTProcessEdges, UnsupportedProcessEdges};
use crate::vm::VMBinding;

pub struct TripleSpaceWorkContext<VM: VMBinding>
(std::marker::PhantomData<VM>);
impl<VM: VMBinding> crate::scheduler::GCWorkContext for TripleSpaceWorkContext<VM> {
    type VM = VM;
    type PlanType = TripleSpace<VM>;
    type DefaultProcessEdges = SFTProcessEdges<Self::VM>;
    type PinningProcessEdges = UnsupportedProcessEdges<Self::VM>;
}
