//! Windows resource limits (Job Objects).
//!
//! A Job Object is created for every run. The child process is assigned to it immediately after
//! it is spawned, and the Job Object carries the same limits that `setrlimit` provides on Unix:
//!
//! | Limit | Job Object flag |
//! |---|---|
//! | Memory | `JOB_OBJECT_LIMIT_PROCESS_MEMORY` + `JOB_OBJECT_LIMIT_JOB_MEMORY` |
//! | CPU time | `JOB_OBJECT_LIMIT_PROCESS_TIME` |
//! | Processes | `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` |
//! | Kill on exit | `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` |
//!
//! `TerminateJobObject` kills the program *and* everything it spawned, which is the Windows
//! equivalent of killing a Unix process group.

use std::io;
use std::os::windows::io::AsRawHandle;
use std::process::{Child, Command};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
    JOB_OBJECT_LIMIT_PROCESS_TIME,
};

use crate::limits::Limits;

/// `CREATE_NO_WINDOW` — prevents a console window from flashing up.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A configured Windows Job Object.
#[derive(Debug)]
pub struct Sandbox {
    job: HANDLE,
}

// Safety: Windows Job Object HANDLE is safe to send and share across threads.
// Windows OS handles are thread-safe and can be closed, waited on, or terminated from any thread.
unsafe impl Send for Sandbox {}
unsafe impl Sync for Sandbox {}

impl Sandbox {
    /// Creates the Job Object and installs the limits.
    pub fn new(limits: &Limits) -> io::Result<Self> {
        // Safety: `CreateJobObjectW` with a null name and null attributes is the documented way to
        // create an unnamed job.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job == INVALID_HANDLE_VALUE || job.is_null() {
            return Err(io::Error::last_os_error());
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_JOB_MEMORY
            | JOB_OBJECT_LIMIT_PROCESS_TIME
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        info.BasicLimitInformation.ActiveProcessLimit = limits.max_processes as u32;
        // `PerProcessUserTimeLimit` is expressed in 100-nanosecond units.
        info.BasicLimitInformation.PerProcessUserTimeLimit =
            (limits.cpu_seconds as i64).saturating_mul(10_000_000);
        info.ProcessMemoryLimit = limits.memory_bytes as usize;
        info.JobMemoryLimit = limits.memory_bytes as usize;

        // Safety: `info` is a fully initialised structure and its length is passed along.
        let ok = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::addr_of!(info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            let err = io::Error::last_os_error();
            unsafe {
                CloseHandle(job);
            }
            return Err(err);
        }

        Ok(Self { job })
    }

    /// Applies the Windows-specific process creation flags.
    pub fn apply(&self, cmd: &mut Command) -> io::Result<()> {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
        Ok(())
    }

    /// Assigns the freshly spawned child to the Job Object.
    ///
    /// If the assignment fails (for example because the parent process itself lives inside a
    /// restrictive job) the run continues without the Job Object limits; the wall-clock watchdog
    /// still applies. The failure is logged so it is visible during development.
    pub fn attach(&self, child: &Child) -> io::Result<()> {
        let handle = child.as_raw_handle() as HANDLE;
        // Safety: `handle` is a live process handle owned by `child` and `self.job` is a live job.
        let ok = unsafe { AssignProcessToJobObject(self.job, handle) };
        if ok == 0 {
            let err = io::Error::last_os_error();
            eprintln!("[sandbox] could not assign process to job object: {err}");
            return Err(err);
        }
        Ok(())
    }

    /// Kills every process inside the Job Object.
    pub fn terminate(&self, _pid: u32) {
        // Safety: the job handle is valid for the lifetime of `self`.
        unsafe {
            TerminateJobObject(self.job, 1);
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // Safety: the handle is closed exactly once, here.
        unsafe {
            CloseHandle(self.job);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_job_object_with_limits() {
        let sandbox = Sandbox::new(&Limits::default()).unwrap();
        assert_ne!(sandbox.job, INVALID_HANDLE_VALUE);
    }

    #[test]
    fn attaches_a_child_process_to_the_job() {
        let sandbox = Sandbox::new(&Limits::default()).unwrap();
        let mut child = Command::new("cmd")
            .args(["/C", "echo hello"])
            .spawn()
            .unwrap();
        let result = sandbox.attach(&child);
        child.wait().unwrap();
        assert!(result.is_ok(), "attaching to the job failed: {result:?}");
    }

    #[test]
    fn terminates_every_process_in_the_job() {
        let sandbox = Sandbox::new(&Limits::default()).unwrap();
        let mut child = Command::new("cmd")
            .args(["/C", "pause"])
            .spawn()
            .unwrap();
        sandbox.attach(&child).unwrap();
        sandbox.terminate(child.id());
        let status = child.wait().unwrap();
        assert!(!status.success());
    }
}
