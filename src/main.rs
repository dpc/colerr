extern crate rustc_serialize;
extern crate libc;
#[macro_use]
extern crate mioco;
extern crate nix;
extern crate docopt;
extern crate env_logger;

use docopt::Docopt;
use std::os::unix::io::FromRawFd;
use std::fs;
use std::io;
use libc::c_int;
use std::ffi::CString;
use nix::unistd::execvp;
use nix::fcntl::{fcntl, FcntlArg, O_NONBLOCK};

mod iomuxer;

static USAGE: &'static str = "
Usage:
    colorout [--] <cmd>...
";

#[derive(Copy, Clone, Debug)]
pub struct Fd(c_int);

impl Fd {

    pub fn raw(&self) -> c_int {
        let Fd(fd) = *self;

        fd
    }

    pub fn close(&self) -> io::Result<()> {
        match unsafe { libc::close(self.raw()) } {
            err if err < 0 => {
                Err(io::Error::from_raw_os_error(err))
            },
            _ => Ok(())
        }
    }

    pub fn to_file(&self) -> fs::File {
        unsafe { fs::File::from_raw_fd(self.raw()) }
    }


    pub fn dup_as(&self, to : Fd) -> io::Result<()> {
        match unsafe { libc::dup2(self.raw(), to.raw()) } {
            err if err < 0 => {
                Err(io::Error::from_raw_os_error(err))
            },
            _ => Ok(())
        }
    }

    pub fn set_nonblocking(&self) {
        fcntl(self.raw(), FcntlArg::F_SETFL(O_NONBLOCK)).expect("fcntl");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FdPipe {
    rx : Fd,
    tx : Fd,
}

impl FdPipe {
    fn new() -> FdPipe {
        let mut fds = [0 as c_int, 0 as c_int];

        let ret = unsafe { libc::pipe(fds.as_mut_ptr())};
        if ret < 0 {
            panic!("unistd::pipe failed: {}", ret);
        }

        FdPipe { rx: Fd(fds[0]), tx: Fd(fds[1]) }
    }

    fn rx(&self) -> Fd {
        self.rx
    }

    fn tx(&self) -> Fd {
        self.tx
    }
}

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_cmd: Vec<String>,
}

fn main() {
    env_logger::init().unwrap();

    let args : Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let stdout_pipe = FdPipe::new();
    let stderr_pipe = FdPipe::new();
    let stdin_pipe = FdPipe::new();

    let child_pid = unsafe { libc::fork() };

    if child_pid == 0 {
        // Output colorizing child
        stdin_pipe.rx().close().unwrap();
        stdout_pipe.tx().close().unwrap();
        stderr_pipe.tx().close().unwrap();

        iomuxer::start(stdin_pipe, stdout_pipe, stderr_pipe);

    } else {
        // The program to be run
        stdin_pipe.tx().close().unwrap();
        stdout_pipe.rx().close().unwrap();
        stderr_pipe.rx().close().unwrap();

        stdin_pipe.rx().dup_as(Fd(0)).unwrap();
        stdout_pipe.tx().dup_as(Fd(1)).unwrap();
        stderr_pipe.tx().dup_as(Fd(2)).unwrap();

        stdin_pipe.rx().close().unwrap();
        stdout_pipe.tx().close().unwrap();
        stderr_pipe.tx().close().unwrap();

        let cmd = CString::new(args.arg_cmd[0].as_str()).unwrap();
        let args_iter = args.arg_cmd.iter();
        let args : Vec<CString> = args_iter.map(|s| CString::new(s.as_str()).unwrap()).collect();

        execvp(
            &cmd,
            args.as_slice(),
            ).expect("execve failed");
        }
}
