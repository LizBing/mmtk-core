use super::Soda;
use crate::scheduler::gc_work::{SFTProcessEdges, UnsupportedProcessEdges};
use crate::vm::VMBinding;

pub struct SodaWorkContext<VM: VMBinding>
(std::marker::PhantomData<VM>);
impl<VM: VMBinding> crate::scheduler::GCWorkContext for SodaWorkContext<VM> {
    type VM = VM;
    type PlanType = Soda<VM>;
    type DefaultProcessEdges = SFTProcessEdges<Self::VM>;
    type PinningProcessEdges = UnsupportedProcessEdges<Self::VM>;
}
