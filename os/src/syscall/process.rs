//! Process management syscalls
#[allow(unused)]
use crate::{
    config::{MAX_SYSCALL_NUM,PAGE_SIZE}, 
    mm::{MapPermission, PageTable, StepByOne, VirtAddr,trans_addr_v2p}, 
    task::{
        TASK_MANAGER,change_program_brk, current_user_token, exit_current_and_run_next, insert_framed_area, suspend_current_and_run_next, unmap_framed_area, TaskStatus
    },//...
    timer::{get_time_ms, get_time_us},
};


#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {//来自用户态的指针
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let ts = trans_addr_v2p(current_user_token(), _ts as usize) as *mut TimeVal;
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/*参考。。。
pub fn get_time() -> isize {
    let mut time = TimeVal::new();
    match sys_get_time(&mut time, 0) {
        0 => ((time.sec & 0xffff) * 1000 + time.usec / 1000) as isize,
        _ => -1,
    }
}
*/

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let ti = trans_addr_v2p(current_user_token(), _ti as usize) as *mut TaskInfo;
    unsafe {
        *ti = TaskInfo {
            status: TaskStatus::Running,
            
            syscall_times: TASK_MANAGER.get_syscall_times(),
            
            time: get_time_ms()-TASK_MANAGER.get_first_time_run(),
        };
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    
    /*start 需要映射的虚存起始地址，要求按页对齐
    len 映射字节长度，可以为 0
    port：第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0

 */
    trace!("kernel: sys_mmap");
    let va_start=VirtAddr::from(start);
    let va_end=VirtAddr::from(start+len);
    if va_start.page_offset()!=0 {return  -1;}//按页对其
    if port & !0x7 != 0||port & 0x7 == 0{
        return -1;
    };//。。。
    let mut  vpn=va_start.floor();
    let user_pt=PageTable::from_token(current_user_token());//现在这里需要获取用户的页表/*调用者（进程的satp） */
    for _ in 0..((len + PAGE_SIZE - 1) / PAGE_SIZE){//遍历即将映射的页
        match user_pt.translate(vpn){
            Some(pte)=>{
                if pte.is_valid(){
                    return -1;//范围内存在已经被映射的页
                }
            }
            None=>{}
        }
        //下一个！
        vpn.step();
    }//这很奇怪
    //接下来是权限
    //port：第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0
    let mut perm=MapPermission::empty();
    
    perm.set(MapPermission::R, port&0x1!=0);//001
    perm.set(MapPermission::W, port&0x2!=0);//01?
    perm.set(MapPermission::X, port&0x4!=0);//1??
    perm.set(MapPermission::U, true); //废话
    /*。。。。*/
    /*先去给应用的页表做点啥 */
    insert_framed_area(va_start,va_end,perm);
    0
}

// YOUR JOB: Implement munmap.
#[allow(unused)]
pub fn sys_munmap(start: usize, len: usize) -> isize {
    
    trace!("kernel: sys_munmap");
    let va_start=VirtAddr::from(start);
    let va_end=VirtAddr::from(start+len);
    if va_start.page_offset()!=0 {return  -1;}//按页对齐
    /*可能的错误：[start, start + len) 中存在未被映射的虚存。 */
    let user_pt=PageTable::from_token(current_user_token());
    let mut vpn=va_start.floor();
    for _ in 0..((len + PAGE_SIZE - 1) / PAGE_SIZE){//遍历映射的页
        match user_pt.translate(vpn){
            Some(pte)=>{
                if !pte.is_valid(){
                    return -1;//范围内存在未被映射的页
                }
            }
            None=>{}
        }
        //下一个！
        vpn.step();
    }
    unmap_framed_area(va_start, va_end);
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
