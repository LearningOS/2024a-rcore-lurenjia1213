//! Process management syscalls
use alloc::sync::Arc;
#[allow(unused)]
use crate::{
    config::{MAX_SYSCALL_NUM,PAGE_SIZE},
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str,trans_addr_v2p,MapPermission,VirtAddr,PageTable,StepByOne},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,get_first_time_run,get_syscall_times,insert_framed_area,remove_framed_area
    },
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
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
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

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let ti = trans_addr_v2p(current_user_token(), _ti as usize) as *mut TaskInfo;
    unsafe {
        *ti = TaskInfo {
            status: TaskStatus::Running,
            
            syscall_times: get_syscall_times(),
            
            time: get_time_ms()-get_first_time_run(),
        };
    }
    0
}

/// YOUR JOB: Implement mmap.
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

/// YOUR JOB: Implement munmap.
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
    remove_framed_area(va_start, va_end);
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
