extern crate libc;

use std::os::unix::io::FromRawFd;
use std::fs;
use std::io;
use std::io::{Read};
use libc::funcs::posix88::unistd;
use libc::c_int;

#[derive(Copy, Clone, Debug)]
struct Fd(c_int);

impl Fd {

    pub fn raw(&self) -> c_int {
        let Fd(fd) = *self;

        fd
    }

    pub fn close(&self) -> io::Result<()> {
        match unsafe { unistd::close(self.raw()) } {
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
        match unsafe { unistd::dup2(self.raw(), to.raw()) } {
            err if err < 0 => {
                Err(io::Error::from_raw_os_error(err))
            },
            _ => Ok(())
        }
    }
}


#[derive(Clone, Debug)]
struct FdPipe {
    rx : Fd,
    tx : Fd,
}

impl FdPipe {
    fn new() -> FdPipe {
        let mut fds = [0 as c_int, 0 as c_int];

        let ret = unsafe { unistd::pipe(fds.as_mut_ptr())};
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

fn main() {

    let stdout_pipe = FdPipe::new();
    let stderr_pipe = FdPipe::new();
    let stdin_pipe = FdPipe::new();

    let child_pid = unsafe { unistd::fork() };

    if child_pid == 0 {
        // Stdio colorizing child
        println!("Child here!");
        let mut tin = String::new();


        stdin_pipe.rx().close().unwrap();
        stdout_pipe.tx().close().unwrap();
        stderr_pipe.tx().close().unwrap();

//        let tx_file = stdout_pipe.tx().to_file();
        let mut rx_file = stdout_pipe.rx().to_file();

        let mut buf : [u8; 128] = [0u8; 128];
        'recv: loop {
            match rx_file.read(&mut buf) {
                Ok(0) => break,
                Ok(size) => println!("Received {} bytes from parent", size),
                Err(_) => break 'recv,
            }
        }
        println!("Child done!");
    } else {
        // Original program to be run
        println!("Parent here!");

        stdin_pipe.tx().close().unwrap();
        stdout_pipe.rx().close().unwrap();
        stderr_pipe.rx().close().unwrap();

        stdin_pipe.rx().dup_as(Fd(0)).unwrap();
        stdout_pipe.tx().dup_as(Fd(1)).unwrap();
        stderr_pipe.tx().dup_as(Fd(2)).unwrap();

        stdin_pipe.rx().close().unwrap();
        stdout_pipe.tx().close().unwrap();
        stderr_pipe.tx().close().unwrap();

        println!("Parent line 1!");
        println!("Parent line 2!");
        println!("Parent line 3!");
        println!("Parent done!");
    }
}
