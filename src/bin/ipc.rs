use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

fn write_request_and_shutdown(unix_stream: &mut UnixStream, msg: &[u8]) -> String {
    unix_stream.write(msg).unwrap();
    let mut response = String::new();
    unix_stream.read_to_string(&mut response).unwrap();
    return response;
}

fn main() -> std::io::Result<()> {
    /*
     * See: https://emmanuelbosquet.com/2022/whatsaunixsocket/
     */
    // ipc file location
    let path = "/home/mfreeman/tsuki_nodeapp.world";

    // what to send to ipc node in js
    let b = b"{\"method\": \"eth_getTransactionCount\", \"params\": [[\"0xe7804c37c13166fF0b37F5aE0BB07A3aEbb6e245\", \"latest\"], [\"0x57571d366a00B3389b0aDf30A114BC7DA7a11580\", \"latest\"]]}";

    // connect
    let mut listener = UnixStream::connect(path)?;

    // get server response
    let response = write_request_and_shutdown(&mut listener, b);
    println!("{response}");
    Ok(())
}
