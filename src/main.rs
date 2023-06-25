use tokio::net::unix::pipe::OpenOptions;
use nix::sys::stat::Mode;
use nix::unistd::mkfifo;
use std::io;
use std::fs;
use std::path::Path;
use std::process::Command;
use prost::Message;
use std::io::Cursor;
use home::home_dir;

pub mod raw_packet {
    include!(concat!(env!("OUT_DIR"), "/pipe_rs.raw_packet.rs"));
}

pub fn serialize_packet(raw_packet: &raw_packet::Packet) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.reserve(raw_packet.encoded_len());
    // Unwrap is safe, since we have reserved sufficient capacity in the vector.
    raw_packet.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize_packet(buf: &[u8]) -> Result<raw_packet::Packet, prost::DecodeError> {
    raw_packet::Packet::decode(&mut Cursor::new(buf))
}


pub fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in src.read_dir()? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let home_dir = home_dir().unwrap();
    let fifo_path = Path::new(&home_dir).join("Downloads/packets.pipe");
    let executable_path = "/Applications/MitmproxyAppleTunnel.app/";
    copy_dir(Path::new("./MitmproxyAppleTunnel.app/"), Path::new(executable_path))?;

    // create new fifo and give read, write and execute rights to the owner
    match mkfifo(&fifo_path, Mode::S_IRWXU) {
        Ok(_) => println!("created {:?}", fifo_path),
        Err(err) => println!("Error creating fifo: {}", err),
    }

    Command::new("open")
        .arg("-a")
        .arg(executable_path)
        .arg("--args")
        .arg(&fifo_path)
        .spawn()
        .expect("failed to execute process");

    let rx = OpenOptions::new().open_receiver(&fifo_path).unwrap();
    let tx = OpenOptions::new().open_sender(fifo_path).unwrap();

    // tokio::spawn(async move {
        let mut msg = vec![0; 1024];
        loop {
            println!("waiting for data"); 
            // Wait for the pipe to be readable
            rx.readable().await.unwrap();
            println!("data is ready to be read"); 
            // Try to read data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match rx.try_read(&mut msg) {
                Ok(n) => {
                    println!("read: {:?}", msg);
                    let (splitted_msg, _) = &msg.split_at(n);
                    let raw_packet = deserialize_packet(splitted_msg).unwrap();
                    println!("{:?}", raw_packet.title);
                    msg.truncate(n);
                    break;
                },
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("Error {}", e)
            };
        }
        // Ok(())
    // });

    // tokio::spawn(async move {
    //     tx.writable().await.unwrap();
    //     loop {
    //         match tx.try_write(b"hello world") {
    //             Ok(n) => {
    //                 println!("written: {:?}", n);
    //                 break;
    //             }
    //             Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
    //                 continue;
    //             }
    //             Err(e) => {
    //                 panic!("Error: {}", e)
    //             }
    //         }
    //     }
    // });
    Ok(())
}
