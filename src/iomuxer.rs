use mio::{Token, EventLoop, EventLoopConfig, Handler, EventSet};
use mio::util::Slab;
use mio::unix::{UnixStream};
use std::os::unix::io::FromRawFd;
use std::io;
use nix;
use std::os::unix::io::AsRawFd;

use mioco;
use mioco::TypedIOHandle;

use FdPipe;
use Fd;

struct IOMuxer {
    parent_stdin : Fd,
    parent_stdout: Fd,
    parent_stderr: Fd,
    own_stdin : Fd,
    own_stdout: Fd,
    conns: Slab<mioco::IOHandle>,
    running: i32,
}

impl IOMuxer {
    fn new(parent_stdin : Fd, parent_stdout : Fd, parent_stderr : Fd) -> IOMuxer {
        IOMuxer {
            parent_stdin : parent_stdin,
            parent_stdout: parent_stdout,
            parent_stderr: parent_stderr,
            own_stdin: Fd(0),
            own_stdout: Fd(1),
            conns: Slab::new(6),
            running: 0,
        }
    }

    fn start_copier(&mut self, ev: &mut EventLoop<IOMuxer>, from_fd : Fd, to_fd : Fd) -> io::Result<()> {

        let mut builder = mioco::Builder::new();

        let from_fd = from_fd.raw();
        let _from_token = self.conns.insert_with(|token| {
            builder.wrap_io(
                ev,
                unsafe {UnixStream::from_raw_fd(from_fd)},
                token
                )
        }).unwrap();

        let to_fd = to_fd.raw();
        let _to_token = self.conns.insert_with(|token| {
            builder.wrap_io(
                ev,
                unsafe {UnixStream::from_raw_fd(to_fd)},
                token
                )
        }).unwrap();

        let f = move |io : &mut mioco::CoroutineHandle| {

            let mut from : TypedIOHandle<UnixStream> = io.handle(0);
            let mut to : TypedIOHandle<UnixStream> = io.handle(1);

            let _ = io::copy(&mut from, &mut to);

            to.with_raw_mut(|io| nix::unistd::close(io.as_raw_fd()).expect("close()"))
        };

        builder.start(f, ev);

        Ok(())
    }

    fn start_muxing_copier(&mut self, ev: &mut EventLoop<IOMuxer>,
                           from1_fd : Fd, from2_fd : Fd, to_fd : Fd) -> io::Result<()> {

        let mut builder = mioco::Builder::new();

        let from1_fd = from1_fd.raw();
        let _ = self.conns.insert_with(|token| {
            builder.wrap_io(
                ev,
                unsafe {UnixStream::from_raw_fd(from1_fd)},
                token
                )
        }).unwrap();

        let from2_fd = from2_fd.raw();
        let _ = self.conns.insert_with(|token| {
            builder.wrap_io(
                ev,
                unsafe {UnixStream::from_raw_fd(from2_fd)},
                token
                )
        }).unwrap();

        let to_fd = to_fd.raw();
        let _ = self.conns.insert_with(|token| {
            builder.wrap_io(
                ev,
                unsafe {UnixStream::from_raw_fd(to_fd)},
                token
                )
        }).unwrap();

        let f = move |io : &mut mioco::CoroutineHandle| {
            use std::io::{Read, Write};
            let mut buf = [0u8; 1024];
            let mut last_source = 0xffff;
            let mut from0 : TypedIOHandle<UnixStream> = io.handle(0);
            let mut from1 : TypedIOHandle<UnixStream> = io.handle(1);
            let mut to : TypedIOHandle<UnixStream> = io.handle(2);
            loop {
                let source = io.select_read_from(&[0, 1]).idx();

                let mut changed = false;

                if last_source != source {
                    last_source = source;
                    changed = true;
                }

                if changed {
                    if let Err(_) = to.write_all(match source {
                        0 => "\x1b[0m",
                        1 => "\x1b[31m",
                        _ => panic!("wrong source"),
                    }.as_bytes()) {
                        break;
                    }
                }

                let res = match source {
                    0 => &mut from0,
                    1 => &mut from1,
                    _ => panic!(),
                }.read(&mut buf);

                match res {
                    Err(_) => break,
                    Ok(0) => /* EOF */ break,
                    Ok(size) => {
                        match to.write_all(&mut buf[0..size]) {
                            Ok(()) => { },
                            Err(_) => { break },
                        }
                    }
                }
            }

            let _ = to.write_all("\x1b[0m".as_bytes());
        };

        builder.start(f, ev);

        Ok(())
    }
    fn start(&mut self, ev: &mut EventLoop<IOMuxer>) -> io::Result<()> {
        let oi = self.own_stdin;
        let pi = self.parent_stdin;
        try!(self.start_copier(ev, oi, pi));

        let oo = self.own_stdout;
        let po = self.parent_stdout;
        let pe = self.parent_stderr;
        try!(self.start_muxing_copier(ev, po, pe, oo));

        self.running = 2;
        Ok(())
    }

    fn conn_handle_finished(&mut self, event_loop: &mut EventLoop<IOMuxer>, finished : bool) {
        if finished {
            self.running -= 1;
            if self.running == 0 {
                event_loop.shutdown()
            }
        }
    }

    fn conn_ready(&mut self, event_loop: &mut EventLoop<IOMuxer>, tok: Token, events : EventSet) {
        let finished = {
            let conn = self.conn(tok);
            conn.ready(event_loop, tok, events);
            conn.is_finished()
        };
        self.conn_handle_finished(event_loop, finished);
    }

    fn conn<'a>(&'a mut self, tok: Token) -> &'a mut mioco::IOHandle {
        &mut self.conns[tok]
    }
}

impl Handler for IOMuxer {
    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<IOMuxer>, token: Token, events: EventSet) {
        self.conn_ready(event_loop, token, events);
    }
}

pub fn start(parent_stdin : FdPipe, parent_stdout : FdPipe, parent_stderr : FdPipe) {
    let config = EventLoopConfig {
        io_poll_timeout_ms: 1,
        notify_capacity: 4_096,
        messages_per_tick: 256,
        timer_tick_ms: 1,
        timer_wheel_size: 1_024,
        timer_capacity: 65_536,
    };

    parent_stdin.tx.set_nonblocking();
    parent_stdout.rx.set_nonblocking();
    parent_stderr.rx.set_nonblocking();
    Fd(0).set_nonblocking();
    Fd(1).set_nonblocking();
    let mut iomuxer = IOMuxer::new(
        parent_stdin.tx,
        parent_stdout.rx,
        parent_stderr.rx
        );

    let mut ev_loop : EventLoop<IOMuxer> = EventLoop::configured(config).expect("EventLoop::configured()");
    iomuxer.start(&mut ev_loop).expect("iomuxer.start");
    ev_loop.run(&mut iomuxer).expect("ev_loop.run()");
}
