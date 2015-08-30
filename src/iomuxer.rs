use mio::unix::{UnixStream};
use std::os::unix::io::{FromRawFd, AsRawFd};
use std::io;
use nix;

use mioco;

use FdPipe;
use Fd;

pub fn start(parent_stdin : FdPipe, parent_stdout : FdPipe, parent_stderr : FdPipe) {
    let parent_stdin  = parent_stdin.tx;
    let parent_stdout = parent_stdout.rx;
    let parent_stderr = parent_stderr.rx;
    let own_stdin = Fd(0);
    let own_stdout = Fd(1);

    parent_stdin.set_nonblocking();
    parent_stdout.set_nonblocking();
    parent_stdout.set_nonblocking();
    own_stdin.set_nonblocking();
    own_stdout.set_nonblocking();

    mioco::start(move |mioco| {
        mioco.spawn(move |mioco| {
            let mut from = mioco.wrap(unsafe {UnixStream::from_raw_fd(own_stdin.raw())});
            let mut to = mioco.wrap(unsafe {UnixStream::from_raw_fd(parent_stdin.raw())});
            try!(io::copy(&mut from, &mut to));
            to.with_raw_mut(|io| nix::unistd::close(io.as_raw_fd()).expect("close()"));
            Ok(())
        });

        mioco.spawn(move |mioco| {
            use std::io::{Read, Write};

            let mut buf = [0u8; 1024];
            let mut from0 = mioco.wrap(unsafe {UnixStream::from_raw_fd(parent_stdout.raw())});
            let mut from1 = mioco.wrap(unsafe {UnixStream::from_raw_fd(parent_stderr.raw())});
            let mut to = mioco.wrap(unsafe {UnixStream::from_raw_fd(own_stdout.raw())});
            let mut last_source = from0.id();

            let _ : io::Result<()> = (|| {
                loop {
                    let source = mioco.select_read_from(&[from0.id(), from1.id()]).id();

                    let mut changed = false;

                    if last_source != source {
                        last_source = source;
                        changed = true;
                    }

                    if changed {
                        if let Err(_) = to.write_all(
                            if source == from0.id() {
                                "\x1b[0m"
                            } else if source == from1.id() {
                                "\x1b[31m"
                            } else {
                                panic!("wrong source")
                            }.as_bytes()
                        ) {
                            break;
                        }
                    }

                    let res = if source == from0.id() {
                        &mut from0
                    } else if source == from1.id() {
                        &mut from1
                    } else {
                        panic!()
                    }.read(&mut buf);

                    match res {
                        Err(_) => break,
                        Ok(0) => /* EOF */ break,
                        Ok(size) => {
                            try!(to.write_all(&mut buf[0..size]));
                        }
                    }
                    let _ = try!(to.write_all("\x1b[0m".as_bytes()));
                }
                Ok(())
            })();

            let _ = try!(to.write_all("\x1b[0m".as_bytes()));

            Ok(())
        });
        Ok(())
    });
}
