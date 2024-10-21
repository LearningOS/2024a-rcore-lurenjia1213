//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{kernel_stack_position, MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry,};
pub use page_table::{PTEFlags, PageTable};
pub use page_table::trans_addr_v2p; 
/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();//初始化rustlang的堆分配器
    frame_allocator::init_frame_allocator();//StackFrame
    KERNEL_SPACE.exclusive_access().activate();//分页，启动！
}
