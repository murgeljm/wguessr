use beef_messages::BeefMessage;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::time::{SystemTime, UNIX_EPOCH};

// I first did it using a trait GenericStream: Read + Write + Sync + Send but this seems simpler
// we defined a wrapper enum and some matches to call underlying functions. The downside is that
// for every new extension we need to define many functions
pub enum GenericStream {
    TcpStream(TcpStream),
    UnixStream(UnixStream),
}

impl Read for GenericStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            GenericStream::TcpStream(s) => s.read(buf),
            GenericStream::UnixStream(s) => s.read(buf),
        }
    }
}

impl Write for GenericStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            GenericStream::TcpStream(s) => s.write(buf),
            GenericStream::UnixStream(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            GenericStream::TcpStream(s) => s.flush(),
            GenericStream::UnixStream(s) => s.flush(),
        }
    }
}

impl GenericStream {
    pub fn send_bytes(&self, bytes: &[u8]) {
        self.get_clone().write_all(bytes).unwrap()
    }
    pub fn send_msg_string(&self, msg: String) {
        self.send_msg(msg.as_str());
    }
    pub fn send_msg(&self, msg: &str) {
        self.send_bytes(format!("{msg}\r\n").as_bytes());
    }
    pub fn receive_msg(&self) -> Result<BeefMessage, ()> {
        let mut reader = BufReader::new(self.get_clone());
        let received: Vec<u8> = reader.fill_buf().unwrap().to_vec();
        reader.consume(received.len());
        Ok(received.into())
    }

    pub fn get_unique_string(&self) -> String {
        match self {
            GenericStream::TcpStream(s) => s.peer_addr().map(|addr| addr.to_string()).unwrap(),
            GenericStream::UnixStream(_) => {
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();
                format!("{}", nanos)
            }
        }
    }

    pub fn get_clone(&self) -> Self {
        match self {
            GenericStream::TcpStream(s) => GenericStream::TcpStream(s.try_clone().unwrap()),
            GenericStream::UnixStream(s) => GenericStream::UnixStream(s.try_clone().unwrap()),
        }
    }
}
